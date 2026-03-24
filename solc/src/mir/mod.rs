use crate::ast::{BinOpKind, UnaryOpKind};
use crate::type_checker::DefId;

mod builder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(usize);

#[derive(Debug)]
pub enum Constant {
    Int(i128),
    Bool(bool),
    Str(String),
    Unit,
}

#[derive(Debug)]
pub enum Operand {
    Temporary(TempId),
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
    body: Vec<Instruction>,
    term: Terminator,
}

#[derive(Debug)]
pub struct Proc {
    temps: Vec<TempId>,
    blocks: Vec<Block>,
}

#[derive(Debug)]
pub struct Module {
    procs: Vec<Proc>,
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
