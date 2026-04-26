use std::collections::HashMap;

use crate::interner::{self, Id};
use crate::type_checker::ty::FloatTy;
use crate::type_checker::{IntTy, Ty, TypeId, UIntTy};

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
    pub const F16: TypeId = TypeId(8);
    pub const F32: TypeId = TypeId(9);
    pub const F64: TypeId = TypeId(10);
    pub const BOOL: TypeId = TypeId(11);
    pub const STR: TypeId = TypeId(12);
    pub const UNIT: TypeId = TypeId(13);
}

#[derive(Debug)]
pub struct TypeInterner {
    idx: u32,
    lookup: HashMap<Ty, TypeId>,
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
        Self {
            idx: 14,
            lookup: HashMap::default(),
        }
    }
}

impl interner::Strategy<TypeId, Ty> for TypeInterner {
    fn key_for(&mut self, value: &Ty) -> TypeId {
        match value {
            Ty::Unit => TypeId::UNIT,
            Ty::Int(int_ty) => match int_ty {
                IntTy::I8 => TypeId::I8,
                IntTy::I16 => TypeId::I16,
                IntTy::I32 => TypeId::I32,
                IntTy::I64 => TypeId::I64,
            },
            Ty::UInt(uint_ty) => match uint_ty {
                UIntTy::U8 => TypeId::U8,
                UIntTy::U16 => TypeId::U16,
                UIntTy::U32 => TypeId::U32,
                UIntTy::U64 => TypeId::U64,
            },
            Ty::Float(float_ty) => match float_ty {
                FloatTy::F16 => TypeId::F16,
                FloatTy::F32 => TypeId::F32,
                FloatTy::F64 => TypeId::F64,
            }
            Ty::Bool => TypeId::BOOL,
            Ty::Str => TypeId::STR,
            Ty::List(..) | Ty::Ptr(_) | Ty::Fn { .. } => {
                if let Some(id) = self.lookup.get(value).copied() {
                    id
                } else {
                    let id = self.next();
                    self.lookup.insert(value.clone(), id);
                    id
                }
            }
            Ty::Struct { .. } => self.next(),
        }
    }

    fn default_values() -> Option<std::collections::HashMap<TypeId, Ty>> {
        Some(
            [
                (TypeId::UNIT, Ty::Unit),
                (TypeId::I8, Ty::Int(IntTy::I8)),
                (TypeId::U8, Ty::UInt(UIntTy::U8)),
                (TypeId::I16, Ty::Int(IntTy::I16)),
                (TypeId::U16, Ty::UInt(UIntTy::U16)),
                (TypeId::I32, Ty::Int(IntTy::I32)),
                (TypeId::U32, Ty::UInt(UIntTy::U32)),
                (TypeId::I64, Ty::Int(IntTy::I64)),
                (TypeId::U64, Ty::UInt(UIntTy::U64)),
                (TypeId::F16, Ty::Float(FloatTy::F16)),
                (TypeId::F32, Ty::Float(FloatTy::F32)),
                (TypeId::F64, Ty::Float(FloatTy::F64)),
                (TypeId::BOOL, Ty::Bool),
                (TypeId::STR, Ty::Str),
            ]
            .into_iter()
            .collect(),
        )
    }
}
