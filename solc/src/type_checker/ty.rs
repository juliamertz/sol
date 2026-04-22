use crate::ast;
use crate::ext::AsStr;
use crate::hir::FieldId;
use crate::interner::Id;
use crate::type_checker::TypeId;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructTy {
    pub ident: Box<ast::Ident>,
    pub fields: Box<[(ast::Name, TypeId)]>,
}

impl StructTy {
    pub fn get_field(&self, name: impl AsStr) -> Option<FieldId> {
        let key = name.as_str();
        self.fields
            .iter()
            .enumerate()
            .find(|(_, (name, _))| name.as_str() == key)
            .map(|(id, _)| FieldId::new(id as u32))
    }
}

// TODO: rename to `Ty`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Unit,
    Int(IntTy),
    UInt(UIntTy),
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

impl Type {
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

    pub fn as_struct(&self) -> Option<&StructTy> {
        match self {
            Self::Struct(struct_ty) => Some(struct_ty),
            _ => None,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}
