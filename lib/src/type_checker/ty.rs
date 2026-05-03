use crate::ast;
use crate::traits::AsStr;
use crate::type_checker::{FieldId, TypeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
}

impl IntTy {
    pub fn bits(&self) -> u8 {
        match self {
            IntTy::I8 => 8,
            IntTy::I16 => 16,
            IntTy::I32 => 32,
            IntTy::I64 => 64,
        }
    }
}

impl From<&ast::IntTy> for IntTy {
    fn from(value: &ast::IntTy) -> Self {
        match value {
            ast::IntTy::I8 => IntTy::I8,
            ast::IntTy::I16 => IntTy::I16,
            ast::IntTy::I32 => IntTy::I32,
            ast::IntTy::I64 => IntTy::I64,
        }
    }
}

impl From<&ast::IntTy> for TypeId {
    fn from(value: &ast::IntTy) -> Self {
        match value {
            ast::IntTy::I8 => TypeId::I8,
            ast::IntTy::I16 => TypeId::I16,
            ast::IntTy::I32 => TypeId::I32,
            ast::IntTy::I64 => TypeId::I64,
        }
    }
}

impl std::fmt::Display for IntTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UIntTy {
    U8,
    U16,
    U32,
    U64,
}

impl UIntTy {
    pub fn bits(&self) -> u8 {
        match self {
            UIntTy::U8 => 8,
            UIntTy::U16 => 16,
            UIntTy::U32 => 32,
            UIntTy::U64 => 64,
        }
    }
}

impl From<&ast::UIntTy> for UIntTy {
    fn from(value: &ast::UIntTy) -> Self {
        match value {
            ast::UIntTy::U8 => UIntTy::U8,
            ast::UIntTy::U16 => UIntTy::U16,
            ast::UIntTy::U32 => UIntTy::U32,
            ast::UIntTy::U64 => UIntTy::U64,
        }
    }
}

impl From<&ast::UIntTy> for TypeId {
    fn from(value: &ast::UIntTy) -> Self {
        match value {
            ast::UIntTy::U8 => TypeId::U8,
            ast::UIntTy::U16 => TypeId::U16,
            ast::UIntTy::U32 => TypeId::U32,
            ast::UIntTy::U64 => TypeId::U64,
        }
    }
}

impl std::fmt::Display for UIntTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            UIntTy::U8 => "u8",
            UIntTy::U16 => "u16",
            UIntTy::U32 => "u32",
            UIntTy::U64 => "u64",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatTy {
    F16,
    F32,
    F64,
}

impl From<&ast::FloatTy> for FloatTy {
    fn from(value: &ast::FloatTy) -> Self {
        match value {
            ast::FloatTy::F16 => FloatTy::F16,
            ast::FloatTy::F32 => FloatTy::F32,
            ast::FloatTy::F64 => FloatTy::F64,
        }
    }
}

impl From<&ast::FloatTy> for TypeId {
    fn from(value: &ast::FloatTy) -> Self {
        match value {
            ast::FloatTy::F16 => TypeId::F16,
            ast::FloatTy::F32 => TypeId::F32,
            ast::FloatTy::F64 => TypeId::F64,
        }
    }
}

impl std::fmt::Display for FloatTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FloatTy::F16 => "f16",
            FloatTy::F32 => "f32",
            FloatTy::F64 => "f64",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructTy {
    pub ident: Box<ast::Ident>,
    pub fields: Box<[(ast::Name, TypeId)]>,
}

impl StructTy {
    pub fn get_field(&self, name: impl AsStr) -> Option<(FieldId, TypeId)> {
        let key = name.as_str();
        self.fields
            .iter()
            .enumerate()
            .find(|(_, (name, _))| name.as_str() == key)
            .map(|(id, (_, ty_id))| (FieldId::from(id), *ty_id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ty {
    Unit,
    Int(IntTy),
    UInt(UIntTy),
    Float(FloatTy),
    Bool,
    Str,
    List(TypeId, Option<usize>), // TODO: probably want to just have this be a sized array, then implement lists in the stdlib?
    Ptr(TypeId),
    Fn {
        is_extern: bool,
        is_variadic: bool,
        params: Box<[TypeId]>,
        returns: TypeId,
    },
    Struct(StructTy),
}

impl Ty {
    pub fn func(params: impl Into<Box<[TypeId]>>, returns: TypeId) -> Self {
        Self::Fn {
            is_extern: false,
            is_variadic: false,
            params: params.into(),
            returns,
        }
    }

    pub fn extern_func(
        params: impl Into<Box<[TypeId]>>,
        returns: TypeId,
        is_variadic: bool,
    ) -> Self {
        Self::Fn {
            is_extern: true,
            is_variadic,
            params: params.into(),
            returns,
        }
    }

    pub fn must_allocate(&self) -> bool {
        matches!(self, Self::Struct(_) | Self::List(_, _))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Self::Int(_) | Self::UInt(_) | Self::Float(_))
    }

    pub fn as_struct(&self) -> Option<&StructTy> {
        match self {
            Self::Struct(struct_ty) => Some(struct_ty),
            _ => None,
        }
    }

    pub fn as_ptr_inner(&self) -> Option<&TypeId> {
        match self {
            Ty::Ptr(inner) => Some(inner),
            _ => None,
        }
    }
}
