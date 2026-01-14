use crate::parser::ast;
use crate::type_checker::{Scope, TypeEnv};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignedIntKind {
    I8,
    I16,
    I32,
    I64,
}

impl SignedIntKind {
    pub fn bits(&self) -> u8 {
        match self {
            SignedIntKind::I8 => 8,
            SignedIntKind::I16 => 16,
            SignedIntKind::I32 => 32,
            SignedIntKind::I64 => 64,
        }
    }
}

impl From<&ast::SignedIntTy> for SignedIntKind {
    fn from(value: &ast::SignedIntTy) -> Self {
        match value {
            ast::SignedIntTy::I8 => SignedIntKind::I8,
            ast::SignedIntTy::I16 => SignedIntKind::I16,
            ast::SignedIntTy::I32 => SignedIntKind::I32,
            ast::SignedIntTy::I64 => SignedIntKind::I64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsignedIntKind {
    U8,
    U16,
    U32,
    U64,
}

impl UnsignedIntKind {
    pub fn bits(&self) -> u8 {
        match self {
            UnsignedIntKind::U8 => 8,
            UnsignedIntKind::U16 => 16,
            UnsignedIntKind::U32 => 32,
            UnsignedIntKind::U64 => 64,
        }
    }
}

impl From<&ast::UnsignedIntTy> for UnsignedIntKind {
    fn from(value: &ast::UnsignedIntTy) -> Self {
        match value {
            ast::UnsignedIntTy::U8 => UnsignedIntKind::U8,
            ast::UnsignedIntTy::U16 => UnsignedIntKind::U16,
            ast::UnsignedIntTy::U32 => UnsignedIntKind::U32,
            ast::UnsignedIntTy::U64 => UnsignedIntKind::U64,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntKind {
    Signed(SignedIntKind),
    Unsigned(UnsignedIntKind),
}

impl IntKind {
    pub fn bits(&self) -> u8 {
        match self {
            IntKind::Signed(kind) => kind.bits(),
            IntKind::Unsigned(kind) => kind.bits(),
        }
    }

    pub fn is_signed(&self) -> bool {
        matches!(self, IntKind::Signed(_))
    }

    pub fn is_unsigned(&self) -> bool {
        matches!(self, IntKind::Unsigned(_))
    }
}

impl From<&ast::IntTy> for IntKind {
    fn from(value: &ast::IntTy) -> Self {
        match value {
            ast::IntTy::Signed(value) => IntKind::Signed(value.into()),
            ast::IntTy::Unsigned(value) => IntKind::Unsigned(value.into()),
        }
    }
}

impl From<SignedIntKind> for IntKind {
    fn from(value: SignedIntKind) -> Self {
        IntKind::Signed(value)
    }
}

impl From<UnsignedIntKind> for IntKind {
    fn from(value: UnsignedIntKind) -> Self {
        IntKind::Unsigned(value)
    }
}

// TODO:
// refactor to use TypeId's
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    None,
    Int(IntKind),
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
