use std::sync::Arc;

use crate::traits::AsStr;
use crate::type_checker::TypeEnv;
use crate::type_checker::ty::Ty;

/// Type wrapper for rich formatting using information from given [`TypeEnv`]
pub struct FmtTy<'env>(&'env Ty, &'env TypeEnv);

impl<'env> FmtTy<'env> {
    pub fn new(ty: &'env Ty, env: &'env TypeEnv) -> FmtTy<'env> {
        Self(ty, env)
    }
}

impl std::fmt::Display for FmtTy<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let FmtTy(ty, env) = self;
        match ty {
            Ty::Unit => f.write_str("()"),
            Ty::Int(int_ty) => int_ty.fmt(f),
            Ty::UInt(uint_ty) => uint_ty.fmt(f),
            Ty::Float(float_ty) => float_ty.fmt(f),
            Ty::Bool => f.write_str("bool"),
            Ty::Str => f.write_str("str"),
            Ty::List(ty_id, len) => {
                let inner_ty = env.types.get(ty_id);
                write!(f, "[{}; {len:?}]", FmtTy::new(inner_ty, env))
            }
            Ty::Ptr(ty_id) => {
                let inner_ty = env.types.get(ty_id);
                write!(f, "*{}", FmtTy::new(inner_ty, env))
            }
            Ty::Fn { .. } => f.write_str("func"),
            Ty::Struct(struct_ty) => f.write_str(struct_ty.ident.as_str()),
        }
    }
}

/// Formatting output for [`Ty`] generated when [`TyDisplay::new`] is called
pub struct TyDisplay(Arc<str>);

impl TyDisplay {
    pub fn new(ty: &Ty, env: &TypeEnv) -> Self {
        Self(Arc::from(FmtTy::new(ty, env).to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for TyDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Display for TyDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
