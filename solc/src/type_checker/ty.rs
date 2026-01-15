use crate::parser::ast;
use crate::type_checker::{Scope, TypeEnv};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    None,
    Int(IntTy),
    UInt(UIntTy),
    Bool,
    Str,
    List((Box<Type>, Option<usize>)),
    Ptr(Box<Type>),
    Fn {
        is_extern: bool,
        params: Vec<Type>,
        returns: Box<Type>,
    },
    Struct {
        ident: ast::Ident,
        fields: Vec<(ast::Ident, Type)>,
    },
    Var(ast::Ident), // This is a real headache having to resolve this, not sure how to fix....
}

impl Type {
    pub fn resolved(&self, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Self {
        if let Type::Var(ident) = self {
            scope
                .get_type(ident)
                .and_then(|id| env.get_definition(id))
                .unwrap_or(self)
                .clone()
        } else {
            self.clone()
        }
    }
}

impl From<&ast::Ty> for Type {
    fn from(ty: &ast::Ty) -> Self {
        Self::from(&ty.kind)
    }
}

impl From<&ast::TyKind> for Type {
    fn from(kind: &ast::TyKind) -> Self {
        match kind {
            ast::TyKind::Int(kind) => Self::Int(kind.into()),
            ast::TyKind::UInt(kind) => Self::UInt(kind.into()),
            ast::TyKind::Bool => Self::Bool,
            ast::TyKind::Str => Self::Str,
            ast::TyKind::Var(name) => Self::Var(name.clone()),
            ast::TyKind::List { inner, size } => {
                Self::List((Box::new(Self::from(inner.as_ref())), *size))
            }
            ast::TyKind::Fn {
                params,
                returns,
                is_extern,
            } => Self::Fn {
                is_extern: *is_extern,
                params: params.iter().map(Self::from).collect(),
                returns: Box::new(Self::from(returns.as_ref())),
            },
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Var(ident) => f.write_str(ident.as_ref()),
            _ => std::fmt::Debug::fmt(self, f),
        }
    }
}
