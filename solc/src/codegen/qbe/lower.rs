use miette::Diagnostic;
use thiserror::Error;

use crate::ast::BinOpKind;
use crate::codegen::qbe::{
    AbiTy, BaseTy, Block, Cmp, Const, Data, DataItem, DataValue, Definition, ExtTy, Function,
    Ident, Instruction, Jump, Linkage, Module, Operand, Param, RegularParam, Statement,
};
use crate::mir::{self, BlockId};
use crate::num::Signedness;
use crate::type_checker::ty::Type;
use crate::type_checker::{TypeEnv, TypeError, TypeId};

#[derive(Error, Diagnostic, Debug)]
pub enum LowerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Type(#[from] TypeError),
}

pub type Result<T, E = LowerError> = std::result::Result<T, E>;

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

fn temp_name<'a>(temp_id: mir::TempId) -> Ident<'a> {
    let idx = temp_id.inner();
    let mut buf = vec![];
    buf.push(ALPHABET[idx]); // TODO: when idx out of range push n+1
    let name = std::str::from_utf8(&buf).unwrap().to_string();
    Ident::temp(name)
}

fn block_name<'a>(block_id: &mir::BlockId) -> Ident<'a> {
    Ident::block(format!("bb{}", block_id.inner()))
}

fn data_name<'a>(data_id: mir::DataId) -> Ident<'a> {
    let idx = data_id.inner();
    let name = format!("dat_{idx}");
    Ident::global(name)
}

pub struct Builder<'env> {
    pub env: &'env TypeEnv,
}

impl<'env> Builder<'env> {
    pub fn new(env: &'env TypeEnv) -> Self {
        Self { env }
    }

    pub fn lower_data<'a>(&self, data: &'a mir::Data) -> Result<Data<'a>> {
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

    pub fn lower_def<'a>(&self, def: &'a mir::Definition) -> Result<Definition<'a>> {
        Ok(match def {
            mir::Definition::Ty(_) => todo!(),
            mir::Definition::Data(data) => Definition::Data(self.lower_data(data)?),
            mir::Definition::Fn(func) => Definition::Fn(self.lower_func(func)?),
        })
    }

    pub fn lower_module<'a>(&self, module: &'a mir::Module) -> Result<Module<'a>> {
        let mut result = Module::default();

        for def in module.defs.iter() {
            result.defs.push(self.lower_def(def)?);
        }

        Ok(result)
    }

    fn assign<'a>(&self, dest: mir::TempId, ty: BaseTy, instr: Instruction<'a>) -> Statement<'a> {
        Statement::Assign(Operand::Var(temp_name(dest)), ty, instr)
    }

    pub fn lower_instruction<'a>(
        &self,
        func: &'a mir::Fn,
        instruction: &'a mir::Instruction,
    ) -> Result<Statement<'a>> {
        match instruction {
            mir::Instruction::Copy { dest, val } => {
                let ty = self.lower_ty(&func.temp_ty(*dest))?;
                Ok(self.assign(
                    *dest,
                    ty.into_base(),
                    Instruction::Copy(self.lower_operand(val)),
                ))
            }
            mir::Instruction::BinOp { dest, op, lhs, rhs } => {
                let val_ty_id = &func.operand_ty(lhs);
                let val_ty = self.env.types.get(val_ty_id).unwrap();
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
                    Eq | Lt | Gt => {
                        let signedness = match val_ty {
                            Type::Int(_) => Signedness::Signed,
                            Type::UInt(_) => Signedness::Unsigned,
                            _ => unreachable!(),
                        };
                        let ty = self.lower_ty(val_ty_id)?;
                        let cmp = match op {
                            Eq => Cmp::Eq,
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
                Ok(self.assign(*dest, return_ty.into_base(), instr))
            }
            mir::Instruction::UnaryOp {
                dest: _,
                op: _,
                rhs: _,
            } => todo!(),
            mir::Instruction::Call {
                dest,
                def,
                operands,
            } => {
                let name = self.env.def_names.get(def).expect("def name");
                let fn_ty = self
                    .env
                    .definitions
                    .get(def)
                    .and_then(|ty_id| self.env.types.get(ty_id))
                    .expect("def type");

                let Type::Fn {
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
                        let ty_id = func.operand_ty(operand);
                        Ok((self.lower_ty(&ty_id)?, self.lower_operand(operand)))
                    })
                    .collect::<Result<Vec<_>>>()?;

                let variadic_idx = if *is_variadic {
                    Some(param_tys.len() as u64)
                } else {
                    None
                };

                let return_ty = self.lower_ty(&func.temp_ty(*dest))?;
                Ok(self.assign(
                    *dest,
                    return_ty.into_base(),
                    Instruction::Call(name.to_string(), operands, variadic_idx),
                ))
            }
            mir::Instruction::Alloc { dest, ty: _ } => {
                let return_ty = self.lower_ty(&func.temp_ty(*dest))?;
                Ok(self.assign(*dest, return_ty.into_base(), Instruction::Alloc4(12))) // TODO: calculate actual size from ty
            }
            mir::Instruction::Load { dest, addr } => {
                let ty = self.lower_ty(&func.temp_ty(*dest))?;
                let load_ty = self.lower_ty(&func.temp_ty(*dest))?;
                Ok(self.assign(
                    *dest,
                    ty.into_base(),
                    Instruction::Load(load_ty, Operand::Var(temp_name(*addr))),
                ))
            }
            mir::Instruction::Store { addr, val } => {
                let ty = self.lower_ty(&func.operand_ty(val))?;
                Ok(Statement::Volatile(Instruction::Store(
                    ty,
                    Operand::Var(temp_name(*addr)),
                    self.lower_operand(val),
                )))
            }
            mir::Instruction::IndexPtr {
                dest,
                base,
                index,
                elem_ty,
            } => {
                // TODO: this won't work. we somehow need to calculate memory offset here based on the index
                let ty = self.lower_ty(elem_ty)?;
                // TODO: Instruction::Add(self.lower_operand(base), self.lower_operand(index)),
                Ok(self.assign(
                    *dest,
                    ty.into_base(),
                    Instruction::Add(self.lower_operand(base), self.lower_operand(index)),
                ))
            }
        }
    }

    fn lower_const<'a>(&self, constant: &'a mir::Constant) -> Const<'a> {
        match constant {
            mir::Constant::Int(val, _) => Const::int(*val),
            mir::Constant::Bool(_) => todo!(),
            mir::Constant::Unit => todo!(),
        }
    }

    fn lower_operand<'a>(&self, operand: &'a mir::Operand) -> Operand<'a> {
        match operand {
            mir::Operand::Temporary(id) => Operand::Var(temp_name(*id)),
            mir::Operand::Data(id) => Operand::Var(data_name(*id)),
            mir::Operand::Constant(constant) => Operand::Const(self.lower_const(constant)),
        }
    }

    pub fn lower_term<'a>(&self, term: &'a mir::Terminator) -> Result<Jump<'a>> {
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
        &self,
        block: &'a mir::Block,
        func: &'a mir::Fn,
        id: BlockId,
    ) -> Result<Block<'a>> {
        let ident = block_name(&id);
        let instructions = block
            .body
            .iter()
            .map(|instr| self.lower_instruction(func, instr))
            .collect::<Result<Vec<_>>>()?;
        let jump = self.lower_term(&block.term)?;

        Ok(Block {
            ident,
            phi_instructions: vec![], // TODO:
            instructions,
            jump,
        })
    }

    pub fn lower_ty<'a>(&self, type_id: &TypeId) -> Result<AbiTy<'a>> {
        let ty = self.env.type_by_id(type_id)?;
        Ok(match ty {
            Type::Unit => AbiTy::Base(BaseTy::Word), // TODO: maybe use a custom unit type
            Type::Int(_) | Type::UInt(_) => AbiTy::Base(BaseTy::Word),
            Type::Bool => AbiTy::Base(BaseTy::Word),
            //not sure if this is correct but it works for data pointers
            Type::Str => AbiTy::Base(BaseTy::Long),
            // List type should also just be a pointer im guessing?
            Type::List(_ty, _size) => AbiTy::Base(BaseTy::Long),
            Type::Ptr(_type_id) => todo!(),
            Type::Fn {
                is_extern: _,
                is_variadic: _,
                params: _,
                returns: _,
            } => todo!(),
            Type::Struct {
                ident: _,
                fields: _,
            } => todo!(),
        })
    }

    pub fn lower_func<'a>(&self, func: &'a mir::Fn) -> Result<Function<'a>> {
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
                .map(|type_id| {
                    Ok(Param::Regular(RegularParam(
                        self.lower_ty(type_id)?,
                        Operand::Var(Ident::temp("a")),
                    )))
                })
                .collect::<Result<Vec<_>>>()?,
            blocks: func
                .blocks
                .iter()
                .enumerate()
                .map(|(idx, block)| self.lower_block(block, func, BlockId::new(idx)))
                .collect::<Result<Vec<_>>>()?,
        })
    }
}
