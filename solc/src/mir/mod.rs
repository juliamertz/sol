use crate::ast::{BinOpKind, UnaryOpKind};
use crate::type_checker::ty::Type;
use crate::type_checker::{DefId, TypeId};

mod builder;
mod fmt;
mod lower;

pub use lower::lower_module;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempId(usize);

impl TempId {
    pub fn inner(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataId(usize);

impl DataId {
    pub fn inner(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(usize);

impl BlockId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn inner(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone)]
pub enum Constant {
    Int(i128),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Temporary(TempId),
    Data(DataId),
    Constant(Constant),
}

#[derive(Debug)]
pub enum Instruction {
    Copy {
        dest: TempId,
        val: Operand,
    },
    BinOp {
        dest: TempId,
        op: BinOpKind,
        lhs: Operand,
        rhs: Operand,
    },
    UnaryOp {
        dest: TempId,
        op: UnaryOpKind,
        rhs: Operand,
    },
    Call {
        dest: TempId,
        def: DefId,
        operands: Vec<Operand>,
    },
}

#[derive(Debug)]
pub enum Terminator {
    Goto(BlockId),
    Return(Operand),
    Branch {
        cond: Operand,
        consequence: BlockId,
        alternative: BlockId,
    },
}

#[derive(Debug)]
pub struct Block {
    pub body: Vec<Instruction>,
    pub term: Terminator,
}

#[derive(Debug)]
pub struct Fn {
    pub name: String,
    pub return_ty: TypeId,
    pub params: Vec<TypeId>,
    pub temps: Vec<TypeId>,
    pub blocks: Vec<Block>,
}

impl Fn {
    pub fn temp_ty(&self, id: TempId) -> TypeId {
        self.temps[id.inner()]
    }
}

#[derive(Debug)]
pub enum DataValue {
    Bytes(Vec<u8>),
    String(String),
}

#[derive(Debug)]
pub struct Data {
    pub id: DataId,
    pub value: DataValue,
}

#[derive(Debug)]
pub enum Definition {
    Ty(Type),
    Data(Data),
    Fn(Fn),
}

#[derive(Debug)]
pub struct Module {
    pub defs: Vec<Definition>,
}

impl Instruction {
    fn copy(dest: TempId, val: Operand) -> Self {
        Self::Copy { dest, val }
    }

    fn bin_op(dest: TempId, op: BinOpKind, lhs: Operand, rhs: Operand) -> Self {
        Self::BinOp { dest, op, lhs, rhs }
    }

    fn unary_op(dest: TempId, op: UnaryOpKind, rhs: Operand) -> Self {
        Self::UnaryOp { dest, op, rhs }
    }

    fn call(dest: TempId, def: DefId, operands: Vec<Operand>) -> Self {
        Self::Call {
            dest,
            def,
            operands,
        }
    }
}

impl Terminator {
    fn goto(block: BlockId) -> Self {
        Self::Goto(block)
    }

    fn ret(operand: Operand) -> Self {
        Self::Return(operand)
    }

    fn branch(cond: Operand, consequence: BlockId, alternative: BlockId) -> Self {
        Terminator::Branch {
            cond,
            consequence,
            alternative,
        }
    }
}
