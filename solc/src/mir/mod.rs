use crate::ast::{BinOpKind, UnaryOpKind};
use crate::interner::Id;
use crate::type_checker::{DefId, FieldId, TypeId};

mod builder;
mod fmt;
mod lower;

pub use lower::lower_module;

id!(TempId);
id!(DataId);
id!(BlockId);

#[derive(Debug, Clone)]
pub enum Constant {
    Int(i128, MirTy),
    Float(f64, MirTy),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone)]
pub enum Operand {
    Temporary(TempId),
    Data(DataId),
    Constant(Constant),
}

impl Operand {
    pub fn unit() -> Self {
        Self::Constant(Constant::Unit)
    }

    pub fn as_temp(&self) -> Option<&TempId> {
        match self {
            Operand::Temporary(temp_id) => Some(temp_id),
            Operand::Data(_) | Operand::Constant(_) => None,
        }
    }
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
        dest: Option<TempId>,
        def: DefId,
        operands: Vec<Operand>,
    },
    Alloc {
        dest: TempId,
        ty: MirTy,
        count: u64,
    },
    Load {
        dest: TempId,
        addr: TempId,
    },
    Store {
        addr: TempId,
        val: Operand,
    },
    IndexPtr {
        dest: TempId,
        base: Operand,
        index: Operand,
        elem_ty: MirTy,
    },
    FieldPtr {
        dest: TempId,
        lval: Operand,
        field_id: FieldId,
        base_ty: MirTy,
        field_ty: MirTy,
    },
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Indirection {
    #[default]
    None,
    Ptr,
}

impl Indirection {
    pub fn is_ptr(&self) -> bool {
        matches!(self, Self::Ptr)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MirTy {
    pub inner: TypeId,
    pub indirection: Indirection,
}

impl MirTy {
    pub fn new(inner: TypeId) -> Self {
        Self {
            inner,
            indirection: Indirection::None,
        }
    }

    pub fn new_ptr(inner: TypeId) -> Self {
        Self {
            inner,
            indirection: Indirection::Ptr,
        }
    }

    pub fn set_indirection(&mut self, indirection: Indirection) {
        self.indirection = indirection
    }
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
    pub return_ty: MirTy,
    pub params: Vec<(TempId, MirTy)>,
    pub temps: Vec<MirTy>,
    pub blocks: Vec<Block>,
}

impl Fn {
    pub fn temp_ty(&self, id: TempId) -> MirTy {
        self.temps[id.into_inner() as usize]
    }

    pub fn operand_ty(&self, op: &Operand) -> MirTy {
        match op {
            Operand::Temporary(id) => self.temp_ty(*id),
            // TODO: for now, all data is strings
            Operand::Data(_) => MirTy::new(TypeId::STR),
            Operand::Constant(constant) => match constant {
                Constant::Int(_, inner_ty) | Constant::Float(_, inner_ty) => *inner_ty,
                Constant::Bool(_) => MirTy::new(TypeId::BOOL),
                Constant::Unit => MirTy::new(TypeId::UNIT),
            },
        }
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
pub enum TyDef {
    Struct {
        name: String,
        fields: Vec<(FieldId, MirTy)>,
    },
}

#[derive(Debug)]
pub enum Definition {
    Ty(TyDef),
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

    fn copy_non_unit(dest: TempId, val: Operand) -> Option<Self> {
        match val {
            Operand::Constant(Constant::Unit) => None,
            _ => Some(Self::copy(dest, val)),
        }
    }

    fn bin_op(dest: TempId, op: BinOpKind, lhs: Operand, rhs: Operand) -> Self {
        Self::BinOp { dest, op, lhs, rhs }
    }

    fn unary_op(dest: TempId, op: UnaryOpKind, rhs: Operand) -> Self {
        Self::UnaryOp { dest, op, rhs }
    }

    fn call(dest: Option<TempId>, def: DefId, operands: Vec<Operand>) -> Self {
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

    #[allow(unused)]
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
