use crate::ast;
use crate::type_checker::TypeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UIntTy {
    U8,
    U16,
    U32,
    U64,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Unit,
    Int(IntTy),
    UInt(UIntTy),
    Bool,
    Str,
    List(TypeId, Option<usize>),
    Ptr(TypeId),
    Fn {
        is_extern: bool,
        is_variadic: bool,
        params: Box<[TypeId]>,
        returns: TypeId,
    },
    Struct {
        ident: Box<ast::Ident>, // TODO: this field should probably be removed but we're still using it in places to pass around the name
        fields: Box<[(ast::Name, TypeId)]>,
    },
}

impl Type {
    pub fn func(params: impl Into<Box<[TypeId]>>, returns: TypeId) -> Self {
        Self::Fn {
            is_extern: false,
            is_variadic: false,
            params: params.into(),
            returns,
        }
    }

    pub fn extern_func(params: impl Into<Box<[TypeId]>>, returns: TypeId, is_variadic: bool) -> Self {
        Self::Fn {
            is_extern: true,
            is_variadic,
            params: params.into(),
            returns,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
