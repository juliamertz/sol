use crate::interner::{self, Id};
use crate::type_checker::{IntTy, Type, TypeId, UIntTy};

impl TypeId {
    pub const NONE: TypeId = TypeId(0);
    pub const I8: TypeId = TypeId(1);
    pub const U8: TypeId = TypeId(2);
    pub const I16: TypeId = TypeId(3);
    pub const U16: TypeId = TypeId(4);
    pub const I32: TypeId = TypeId(5);
    pub const U32: TypeId = TypeId(6);
    pub const I64: TypeId = TypeId(7);
    pub const U64: TypeId = TypeId(8);
    pub const BOOL: TypeId = TypeId(9);
    pub const STR: TypeId = TypeId(10);
    pub const UNIT: TypeId = TypeId(11);
}

#[derive(Debug)]
pub struct TypeInterner {
    idx: u32,
}

impl TypeInterner {
    fn next(&mut self) -> TypeId {
        let id = TypeId::new(self.idx);
        self.idx += 1;
        id
    }
}

impl Default for TypeInterner {
    fn default() -> Self {
        Self { idx: 12 }
    }
}

impl interner::Strategy<TypeId, Type> for TypeInterner {
    fn key_for(&mut self, value: &Type) -> TypeId {
        match value {
            Type::Unit => TypeId::UNIT,
            Type::Int(int_ty) => match int_ty {
                IntTy::I8 => TypeId::I8,
                IntTy::I16 => TypeId::I16,
                IntTy::I32 => TypeId::I32,
                IntTy::I64 => TypeId::I64,
            },
            Type::UInt(uint_ty) => match uint_ty {
                UIntTy::U8 => TypeId::U8,
                UIntTy::U16 => TypeId::U16,
                UIntTy::U32 => TypeId::U32,
                UIntTy::U64 => TypeId::U64,
            },
            Type::Bool => TypeId::BOOL,
            Type::Str => TypeId::STR,
            Type::List(..) | Type::Ptr(_) | Type::Fn { .. } | Type::Struct { .. } => self.next(),
        }
    }

    fn default_values() -> Option<std::collections::HashMap<TypeId, Type>> {
        Some(
            [
                (TypeId::UNIT, Type::Unit),
                (TypeId::I8, Type::Int(IntTy::I8)),
                (TypeId::U8, Type::UInt(UIntTy::U8)),
                (TypeId::I16, Type::Int(IntTy::I16)),
                (TypeId::U16, Type::UInt(UIntTy::U16)),
                (TypeId::I32, Type::Int(IntTy::I32)),
                (TypeId::U32, Type::UInt(UIntTy::U32)),
                (TypeId::I64, Type::Int(IntTy::I64)),
                (TypeId::U64, Type::UInt(UIntTy::U64)),
                (TypeId::BOOL, Type::Bool),
                (TypeId::STR, Type::Str),
            ]
            .into_iter()
            .collect(),
        )
    }
}
