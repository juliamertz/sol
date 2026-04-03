use miette::Diagnostic;
use thiserror::Error;

use crate::ast::BinOpKind;
use crate::codegen::qbe::{
    AbiTy, BaseTy, Block, Const, Data, DataItem, DataValue, Definition, ExtTy, Function, Ident,
    Instruction, InstructionKind, Jump, Linkage, Module, Operand, Param, RegularParam,
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

    pub fn lower_instruction<'a>(
        &self,
        func: &'a mir::Fn,
        instruction: &'a mir::Instruction,
    ) -> Result<Instruction<'a>> {
        match instruction {
            mir::Instruction::Copy { dest, val } => {
                let return_ty = self.lower_ty(&func.temp_ty(*dest))?;
                Ok(Instruction::copy(
                    temp_name(*dest),
                    return_ty,
                    vec![self.lower_operand(val)],
                ))
            }
            mir::Instruction::BinOp { dest, op, lhs, rhs } => {
                let return_ty = self.lower_ty(&func.temp_ty(*dest))?;
                let val_ty = self.env.types.get(&func.operand_ty(lhs)).unwrap();
                dbg!(&val_ty);
                use BinOpKind::*;
                let kind = match op {
                    Add => Instruction::ADD,
                    Sub => Instruction::SUB,
                    Mul => Instruction::MUL,
                    Div => Instruction::DIV,
                    And => Instruction::AND,
                    Or => Instruction::OR,
                    Eq | Lt | Gt => {
                        let (signedness, bits) = match val_ty {
                            Type::Int(int_ty) => (Signedness::Signed, int_ty.bits()),
                            Type::UInt(uint_ty) => (Signedness::Unsigned, uint_ty.bits()),
                            _ => unreachable!(),
                        };
                        match op {
                            Eq => {
                                if bits == 64 {
                                    Instruction::CEQL
                                } else {
                                    Instruction::CEQW
                                }
                            }
                            Lt => match (signedness, bits == 64) {
                                (Signedness::Signed, false) => Instruction::CSLTW,
                                (Signedness::Unsigned, false) => Instruction::CULTW,
                                (Signedness::Signed, true) => Instruction::CSLTL,
                                (Signedness::Unsigned, true) => Instruction::CULTL,
                            },
                            Gt => match (signedness, bits == 64) {
                                (Signedness::Signed, false) => Instruction::CSGTW,
                                (Signedness::Unsigned, false) => Instruction::CUGTW,
                                (Signedness::Signed, true) => Instruction::CSGTL,
                                (Signedness::Unsigned, true) => Instruction::CUGTL,
                            },
                            _ => unreachable!(),
                        }
                    }
                };
                Ok(Instruction::new(
                    kind,
                    temp_name(*dest),
                    return_ty,
                    vec![self.lower_operand(lhs), self.lower_operand(rhs)],
                ))
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

                let return_ty = self.lower_ty(&func.temp_ty(*dest))?;
                let mut operands = operands
                    .iter()
                    .map(|operand| {
                        let ty_id = func.operand_ty(operand);
                        Ok(Param::Regular(RegularParam(
                            self.lower_ty(&ty_id)?,
                            self.lower_operand(operand),
                        )))
                    })
                    .collect::<Result<Vec<_>>>()?;

                if *is_variadic {
                    let idx = param_tys.len();
                    operands.insert(idx, Param::VariadicMarker);
                }

                Ok(Instruction {
                    ident: temp_name(*dest),
                    return_ty,
                    kind: InstructionKind::Call(Ident::Global(name.to_string().into()), operands),
                })
            }
            mir::Instruction::Alloc { dest, ty } => todo!(),
            mir::Instruction::Load { dest, addr } => todo!(),
            mir::Instruction::Store { addr, val } => todo!(),
            mir::Instruction::IndexPtr { dest, base, index, elem_ty } => todo!(),
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
            Type::Int(_) | Type::UInt(_) => AbiTy::Base(BaseTy::Word), // TODO:
            // Type::Int(int_ty) => match int_ty {
            //     ty::IntTy::I8 => AbiTy::SubWord(SubWordTy::SignedByte),
            //     ty::IntTy::I16 => AbiTy::SubWord(SubWordTy::SingedHalf),
            //     ty::IntTy::I32 => todo!(),
            //     ty::IntTy::I64 => todo!(),
            // },
            // Type::UInt(uint_ty) => todo!(),
            Type::Bool => AbiTy::Base(BaseTy::Word), // TODO:
            Type::Str => AbiTy::Base(BaseTy::Long), // TODO: not sure if this is correct but it works for data pointers
            Type::List(_type_id, _) => todo!(),
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
