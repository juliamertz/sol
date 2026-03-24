use crate::ast::OpKind;
use crate::type_checker::DefId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempId(usize);

/// Indice into a function's blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(usize);

#[derive(Debug)]
pub enum Constant {
    Int(i128),
    Bool(bool),
    Str(String),
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
        op: OpKind,
        lhs: Operand,
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
    Branch {
        cond: TempId,
        consequence: BlockId,
        alternative: BlockId,
    },
    Goto(BlockId),
    Return(Operand),
}
