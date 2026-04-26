use std::collections::HashMap;
use std::rc::Rc;

use miette::Diagnostic;
use thiserror::Error;

use crate::ast::{BinOpKind, UnaryOpKind};
use crate::codegen::qbe::{
    AbiTy, BaseTy, Block, Cmp, Const, Data, DataItem, DataValue, Definition, ExtTy, Function,
    Ident, Instruction, IntoOperand, Jump, Linkage, Module, Operand, Param, Precision,
    RegularParam, Statement, TyDef,
};
use crate::interner::Id;
use crate::mir::{self, BlockId};
use crate::number::Signedness;
use crate::number::encode::bijective_base26;
use crate::traits::AsStr;
use crate::type_checker::ty::{FloatTy, StructTy, Ty};
use crate::type_checker::{TypeEnv, TypeError, TypeId};

#[derive(Error, Diagnostic, Debug)]
pub enum LowerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Type(#[from] TypeError),
}

pub type Result<T, E = LowerError> = std::result::Result<T, E>;

fn block_name<'a>(block_id: &mir::BlockId) -> Ident {
    Ident::block(format!("bb{}", block_id.into_inner()))
}

fn data_name<'a>(data_id: mir::DataId) -> Ident {
    let idx = data_id.into_inner();
    let name = format!("dat_{idx}");
    Ident::global(name)
}

#[derive(Debug, Clone, Copy)]
pub struct TempId(usize);

impl IntoOperand for TempId {
    fn into_operand(self) -> Operand {
        Operand::Var(Ident::temp(format!(
            "_{}",
            bijective_base26(self.0 + (26 * 26))
        )))
    }
}

impl IntoOperand for mir::TempId {
    fn into_operand(self) -> Operand {
        Operand::Var(Ident::temp(bijective_base26(self.into_inner() as usize)))
    }
}

pub struct Builder<'env> {
    pub env: &'env TypeEnv,
    tmp_idx: usize,
    type_defs: HashMap<String, Rc<TyDef>>,
}

impl<'env> Builder<'env> {
    pub fn new(env: &'env TypeEnv) -> Self {
        Self {
            env,
            tmp_idx: 0,
            type_defs: HashMap::default(),
        }
    }

    pub fn new_temp(&mut self) -> TempId {
        let idx = self.tmp_idx;
        self.tmp_idx += 1;
        TempId(idx)
    }

    pub fn lower_ty_def(&self, _ty: &Ty) -> Result<TyDef> {
        // TyDef::Regular { ident: (), align: (), sub_tys: () }
        todo!()
    }

    pub fn lower_data(&self, data: &mir::Data) -> Result<Data> {
        Ok(Data {
            linkage: None,
            ident: data_name(data.id), // TODO: unique idents
            align: None,
            value: DataValue::Data(match &data.value {
                mir::DataValue::Bytes(_items) => todo!(),
                mir::DataValue::String(str) => vec![
                    (ExtTy::Byte, DataItem::String(str.into())),
                    (ExtTy::Byte, DataItem::Const(Const::int(0_i128))),
                ],
            }),
        })
    }

    pub fn lower_def(&mut self, def: &mir::Definition) -> Result<Definition> {
        Ok(match def {
            mir::Definition::Ty(ty) => {
                let (ident, ty_def) = match ty {
                    mir::TyDef::Struct { name, fields } => (
                        name.as_str(),
                        Rc::new(TyDef::Regular {
                            ident: Ident::Ty(name.to_string()),
                            align: None, // TODO:
                            items: fields
                                .iter()
                                .map(|(_, type_id)| {
                                    Ok((
                                        self.lower_ty(type_id).map(|abi_ty| abi_ty.as_sub_ty())?,
                                        1,
                                    ))
                                })
                                .collect::<Result<Vec<_>>>()?,
                        }),
                    ),
                };

                self.type_defs.insert(ident.to_string(), ty_def.clone());

                Definition::Ty(ty_def)
            }
            mir::Definition::Data(data) => Definition::Data(self.lower_data(data)?),
            mir::Definition::Fn(func) => Definition::Fn(self.lower_func(func)?),
        })
    }

    pub fn lower_module(&mut self, module: &mir::Module) -> Result<Module> {
        let mut result = Module::default();

        for def in module.defs.iter() {
            result.defs.push(self.lower_def(def)?);
        }

        Ok(result)
    }

    fn assign(&self, dest: impl IntoOperand, ty: BaseTy, instr: Instruction) -> Statement {
        Statement::Assign(dest.into_operand(), ty, instr)
    }

    fn volatile(&self, instr: Instruction) -> Statement {
        Statement::Volatile(instr)
    }

    pub fn lower_instruction<'a>(
        &mut self,
        func: &'a mir::Fn,
        instruction: &'a mir::Instruction,
    ) -> Result<Vec<Statement>> {
        match instruction {
            mir::Instruction::Copy { dest, val } => {
                let ty = self.lower_ty(&func.temp_ty(*dest))?;
                Ok(vec![self.assign(
                    *dest,
                    ty.as_base(),
                    Instruction::Copy(self.lower_operand(val)),
                )])
            }

            mir::Instruction::BinOp { dest, op, lhs, rhs } => {
                let val_ty_id = &func.operand_ty(lhs);
                let val_ty = self.env.types.get(&val_ty_id.inner);
                let return_ty = self.lower_ty(&func.temp_ty(*dest))?;
                let lhs = self.lower_operand(lhs);
                let rhs = self.lower_operand(rhs);

                use BinOpKind::*;
                let instr = match op {
                    Add => Instruction::Add(lhs, rhs),
                    Sub => Instruction::Sub(lhs, rhs),
                    Mul => Instruction::Mul(lhs, rhs),
                    Div => Instruction::Div(lhs, rhs),
                    And => Instruction::And(lhs, rhs),
                    Or => Instruction::Or(lhs, rhs),
                    Eq | Ne | Lt | Gt => {
                        let signedness = match val_ty {
                            Ty::Int(_) => Signedness::Signed,
                            Ty::UInt(_) => Signedness::Unsigned,
                            _ => unreachable!(),
                        };
                        let ty = self.lower_ty(val_ty_id)?;
                        let cmp = match op {
                            Eq => Cmp::Eq,
                            Ne => Cmp::Ne,
                            Lt => match signedness {
                                Signedness::Signed => Cmp::Slt,
                                Signedness::Unsigned => Cmp::Ult,
                            },
                            Gt => match signedness {
                                Signedness::Signed => Cmp::Sgt,
                                Signedness::Unsigned => Cmp::Ugt,
                            },
                            _ => unreachable!(),
                        };
                        Instruction::Cmp(ty, cmp, lhs, rhs)
                    }
                };
                Ok(vec![self.assign(*dest, return_ty.as_base(), instr)])
            }

            mir::Instruction::UnaryOp { dest, op, rhs } => {
                let return_ty = self.lower_ty(&func.temp_ty(*dest))?;
                let rhs = self.lower_operand(rhs);
                let instr = match op {
                    UnaryOpKind::Negate => Instruction::Neg(rhs),
                    UnaryOpKind::Not => Instruction::Xor(Operand::Const(Const::int(1)), rhs),
                };

                Ok(vec![self.assign(*dest, return_ty.as_base(), instr)])
            }

            mir::Instruction::Call {
                dest,
                def,
                operands,
            } => {
                let name = self.env.def_names.get(def).expect("def name");
                let fn_ty = self.env.types.get(self.env.definitions.get(def));

                let Ty::Fn {
                    is_variadic,
                    params: param_tys,
                    ..
                } = &fn_ty
                else {
                    unreachable!("OH NO your function is not a function? 🤯");
                };

                let operands = operands
                    .iter()
                    .map(|operand| {
                        let ty = func.operand_ty(operand);
                        Ok((self.lower_ty(&ty)?, self.lower_operand(operand)))
                    })
                    .collect::<Result<Vec<_>>>()?;

                let variadic_idx = if *is_variadic {
                    Some(param_tys.len() as u64)
                } else {
                    None
                };

                let instr = Instruction::Call(name.to_string(), operands, variadic_idx);
                match dest {
                    Some(dest) => {
                        let return_ty = self.lower_ty(&func.temp_ty(*dest))?;
                        Ok(vec![self.assign(*dest, return_ty.as_base(), instr)])
                    }
                    None => Ok(vec![self.volatile(instr)]),
                }
            }
            mir::Instruction::Alloc { dest, ty, count } => {
                let ty = self.lower_ty(ty)?;
                Ok(vec![self.assign(
                    *dest,
                    BaseTy::Long,
                    Instruction::Alloc4((ty.size() * count) as u32),
                )])
            }
            mir::Instruction::Load { dest, addr } => {
                let ty = self.lower_ty(&func.temp_ty(*dest))?;
                let load_ty = self.lower_ty(&func.temp_ty(*dest))?;
                Ok(vec![self.assign(
                    *dest,
                    ty.as_base(),
                    Instruction::Load(load_ty, addr.into_operand()),
                )])
            }
            mir::Instruction::Store { addr, val } => {
                let ty = self.lower_ty(&func.operand_ty(val))?;
                Ok(vec![Statement::Volatile(Instruction::Store(
                    ty,
                    addr.into_operand(),
                    self.lower_operand(val),
                ))])
            }
            mir::Instruction::IndexPtr {
                dest,
                base,
                index,
                elem_ty,
            } => {
                let ty = self.lower_ty(elem_ty)?;
                let ptr_offset_dest = self.new_temp();

                Ok(vec![
                    self.assign(
                        ptr_offset_dest,
                        BaseTy::Long,
                        Instruction::Mul(
                            self.lower_operand(index),
                            Operand::Const(Const::int(i128::from(ty.size()))),
                        ),
                    ),
                    self.assign(
                        *dest,
                        ty.as_base(),
                        Instruction::Add(self.lower_operand(base), ptr_offset_dest.into_operand()),
                    ),
                ])
            }

            mir::Instruction::FieldPtr {
                dest,
                lval,
                field_id,
                base_ty,
                field_ty: _,
            } => {
                let base_ty = self.lower_ty(base_ty)?;
                let ty_def = base_ty.as_aggregate().expect("field ptr on tydef");
                let ptr_offset = ty_def.offset_for(field_id.into_inner());

                Ok(vec![self.assign(
                    *dest,
                    BaseTy::Long,
                    Instruction::Add(
                        self.lower_operand(lval),
                        Operand::Const(Const::int(i128::from(ptr_offset))),
                    ),
                )])
            }
        }
    }

    fn lower_const(&self, constant: &mir::Constant) -> Const {
        match constant {
            mir::Constant::Int(val, _) => Const::int(*val),
            mir::Constant::Float(val, mir_ty) => {
                let precision = match mir_ty.inner {
                    TypeId::F16 | TypeId::F32 => Precision::Single,
                    TypeId::F64 => Precision::Double,
                    _ => unreachable!(),
                };
                Const::Float(precision, *val)
            },
            mir::Constant::Bool(_) => todo!(),
            mir::Constant::Unit => unreachable!(), // TODO: this is deffo not unreachable, but ideally it should be :)
        }
    }

    fn lower_operand(&self, operand: &mir::Operand) -> Operand {
        match operand {
            mir::Operand::Temporary(id) => id.into_operand(),
            mir::Operand::Data(id) => Operand::Var(data_name(*id)),
            mir::Operand::Constant(constant) => Operand::Const(self.lower_const(constant)),
        }
    }

    pub fn lower_term(&self, term: &mir::Terminator) -> Result<Jump> {
        Ok(match term {
            mir::Terminator::Goto(block_id) => Jump::Jmp(block_name(block_id)),
            mir::Terminator::Return(operand) => Jump::Ret(self.lower_operand(operand)),
            mir::Terminator::Branch {
                cond,
                consequence,
                alternative,
            } => Jump::Jnz(
                self.lower_operand(cond),
                block_name(consequence),
                block_name(alternative),
            ),
        })
    }

    pub fn lower_block<'a>(
        &mut self,
        block: &'a mir::Block,
        func: &'a mir::Fn,
        id: BlockId,
    ) -> Result<Block> {
        let ident = block_name(&id);
        let instructions = block
            .body
            .iter()
            .map(|instr| self.lower_instruction(func, instr))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        let jump = self.lower_term(&block.term)?;

        Ok(Block {
            ident,
            phi_instructions: vec![], // TODO:
            instructions,
            jump,
        })
    }

    pub fn lower_ty<'a>(&self, mir_ty: &mir::MirTy) -> Result<AbiTy> {
        if mir_ty.indirection.is_ptr() {
            return Ok(BaseTy::Long.into());
        }
        let ty = self.env.type_by_id(&mir_ty.inner)?;
        Ok(match ty {
            Ty::Unit => BaseTy::Word.into(), // TODO: should be omitted
            Ty::Int(_) | Ty::UInt(_) => BaseTy::Word.into(),// TODO: size
            Ty::Float(float_ty) => match float_ty {
                FloatTy::F16 | FloatTy::F32 => BaseTy::Single.into(),
                FloatTy::F64 => BaseTy::Double.into(),
            }
            Ty::Bool => BaseTy::Word.into(),
            Ty::Str => BaseTy::Long.into(),
            Ty::List(_ty, _size) => BaseTy::Long.into(),
            Ty::Ptr(_type_id) => todo!(),
            Ty::Fn {
                is_extern: _,
                is_variadic: _,
                params: _,
                returns: _,
            } => todo!(),
            Ty::Struct(StructTy { ident, .. }) => {
                let Some(ty_def) = self.type_defs.get(ident.as_str()) else {
                    todo!("unknown type: {ident}");
                };

                AbiTy::Aggregate(ty_def.clone())
            }
        })
    }

    pub fn lower_func(&mut self, func: &mir::Fn) -> Result<Function> {
        let linkage = if &func.name == "main" {
            Some(Linkage::Export)
        } else {
            None
        };

        Ok(Function {
            linkage,
            ident: Ident::global(&func.name),
            return_ty: Some(self.lower_ty(&func.return_ty)?),
            params: func
                .params
                .iter()
                .map(|(temp_id, type_id)| {
                    Ok(Param::Regular(RegularParam(
                        self.lower_ty(type_id)?,
                        temp_id.into_operand(),
                    )))
                })
                .collect::<Result<Vec<_>>>()?,
            blocks: func
                .blocks
                .iter()
                .enumerate()
                .map(|(idx, block)| self.lower_block(block, func, BlockId::from(idx)))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}
