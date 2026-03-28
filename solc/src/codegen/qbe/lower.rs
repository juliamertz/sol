use std::collections::HashMap;

use miette::Diagnostic;
use thiserror::Error;

use crate::ast::BinOpKind;
use crate::codegen::qbe::{
    AbiTy, BaseTy, Block, Const, Data, DataItem, DataValue, Definition, ExtTy, Function, Ident,
    Instruction, InstructionKind, Jump, Module, Operand, Param, RegularParam, SubWordTy,
};
use crate::mir::{self, BlockId};
use crate::type_checker::ty::{self, Type};
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
                mir::DataValue::Bytes(items) => todo!(),
                mir::DataValue::String(str) => vec![
                    (ExtTy::Byte, DataItem::String(str.into())),
                    (ExtTy::Byte, DataItem::Const(Const::int(0 as i128))),
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
                let kind = match op {
                    BinOpKind::Eq => Instruction::CEQW, // TODO: correct instr for type
                    BinOpKind::Add => Instruction::ADD,
                    BinOpKind::Sub => Instruction::SUB,
                    BinOpKind::Mul => Instruction::MUL,
                    BinOpKind::Div => Instruction::DIV,
                    BinOpKind::Lt => todo!(),
                    BinOpKind::Gt => todo!(),
                    BinOpKind::And => Instruction::AND,
                    BinOpKind::Or => Instruction::OR,
                };
                Ok(Instruction::new(
                    kind,
                    temp_name(*dest),
                    return_ty,
                    vec![self.lower_operand(lhs), self.lower_operand(rhs)],
                ))
            }
            mir::Instruction::UnaryOp { dest, op, rhs } => todo!(),
            mir::Instruction::Call {
                dest,
                def,
                operands,
            } => {
                let name = self.env.def_names.get(def).expect("def name");
                let return_ty = self.lower_ty(&func.temp_ty(*dest))?;
                let operands = operands
                    .iter()
                    .map(|operand| {
                        RegularParam(AbiTy::Base(BaseTy::Word), self.lower_operand(operand))
                    })
                    .collect();
                Ok(Instruction {
                    ident: temp_name(*dest),
                    return_ty,
                    kind: InstructionKind::Call(Ident::Global(name.to_string().into()), operands),
                })
            }
        }
    }

    fn lower_const<'a>(&self, constant: &'a mir::Constant) -> Const<'a> {
        match constant {
            mir::Constant::Int(val) => Const::int(*val),
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
            Type::Str => todo!(),
            Type::List(type_id, _) => todo!(),
            Type::Ptr(type_id) => todo!(),
            Type::Fn {
                is_extern,
                params,
                returns,
            } => todo!(),
            Type::Struct { ident, fields } => todo!(),
        })
    }

    pub fn lower_func<'a>(&self, func: &'a mir::Fn) -> Result<Function<'a>> {
        Ok(Function {
            linkage: None,
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
