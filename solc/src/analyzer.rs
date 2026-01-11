use std::collections::HashMap;

use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

use crate::ast::{
    BinOp, CallExpr, Constructor, Expr, Fn, Ident, IfElse, Impl, IndexExpr, IntTyKind, Let, List,
    Literal, LiteralKind, Node, NodeId, OpKind, PrefixExpr, Ret, Span, Stmnt, StructDef, Ty,
    TyKind, Use,
};
use crate::source::SourceInfo;
use solc_macros::Id;

#[derive(Debug, Error, Diagnostic)]
pub enum TypeError {
    #[error("variable not found in scope: {0}")]
    NotFound(Ident),

    #[error("invalid type, expected: {expected:?}, got: {actual:?}")]
    InvalidType { expected: Type, actual: Type },

    #[error("mismatched types in comparison")]
    ComparisonMismatch {
        #[source_code]
        src: SourceInfo,

        #[label("has type `{lhs_ty}`")]
        lhs_span: SourceSpan,
        lhs_ty: Type,

        #[label("has type `{rhs_ty}`")]
        rhs_span: SourceSpan,
        rhs_ty: Type,

        #[help]
        help: Option<String>,
    },
}

pub type Result<T, E = TypeError> = core::result::Result<T, E>;

#[derive(Id, Debug, Clone, Copy)]
pub struct DefId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[rustfmt::skip]
pub enum IntKind {
    U8, U16, U32, U64,
    I8, I16, I32, I64,
}

impl IntKind {
    fn is_signed(&self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64)
    }

    fn bits(&self) -> usize {
        match self {
            Self::U8 | Self::I8 => 8,
            Self::U16 | Self::I16 => 16,
            Self::U32 | Self::I32 => 32,
            Self::U64 | Self::I64 => 64,
        }
    }
}

impl From<&IntTyKind> for IntKind {
    fn from(value: &IntTyKind) -> Self {
        match value {
            IntTyKind::U8 => Self::U8,
            IntTyKind::U16 => Self::U16,
            IntTyKind::U32 => Self::U32,
            IntTyKind::U64 => Self::U64,
            IntTyKind::I8 => Self::I8,
            IntTyKind::I16 => Self::I16,
            IntTyKind::I32 => Self::I32,
            IntTyKind::I64 => Self::I64,
        }
    }
}

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
        ident: Ident,
        fields: Vec<(Ident, Type)>,
    },
    Var(Ident),
}

impl From<&Ty> for Type {
    fn from(ty: &Ty) -> Self {
        Self::from(&ty.kind)
    }
}

impl From<&TyKind> for Type {
    fn from(kind: &TyKind) -> Self {
        match kind {
            TyKind::Int(kind) => Self::Int(kind.into()),
            TyKind::Bool => Self::Bool,
            TyKind::Str => Self::Str,
            TyKind::Var(name) => Self::Var(name.clone()),
            TyKind::List { inner, size } => {
                Self::List((Box::new(Self::from(inner.as_ref())), *size))
            }
            TyKind::Fn {
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
            // Type::None => f.write_str("None"),
            // Type::Int(int_kind) => ,
            // Type::Bool => todo!(),
            // Type::Str => todo!(),
            // Type::List(_) => todo!(),
            // Type::Ptr(_) => todo!(),
            // Type::Fn { is_extern, params, returns } => todo!(),
            // Type::Struct { ident, fields } => todo!(),
            Type::Var(ident) => f.write_str(ident.as_ref()),
            _ => std::fmt::Debug::fmt(self, f),
        }
    }
}

#[derive(Debug)]
pub struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    variables: HashMap<String, DefId>,
    src: SourceInfo,
}

impl Scope<'_> {
    pub fn new(src: SourceInfo) -> Self {
        Self {
            parent: None,
            variables: Default::default(),
            src,
        }
    }

    pub fn set_var(&mut self, ident: impl ToString, def_id: DefId) {
        self.variables.insert(ident.to_string(), def_id);
    }

    pub fn get_var(&self, ident: impl AsRef<str>) -> Option<&DefId> {
        self.variables.get(ident.as_ref()).or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_var(ident))
        })
    }

    pub fn child(&self) -> Scope<'_> {
        Scope {
            parent: Some(self),
            variables: Default::default(),
            src: self.src.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct TypeEnv {
    def_idx: u32,
    pub nodes: HashMap<NodeId, Type>,
    pub definitions: HashMap<DefId, Type>,
}

impl TypeEnv {
    pub fn define(&mut self, ty: Type) -> DefId {
        let id = DefId(self.def_idx);
        self.def_idx += 1;
        self.definitions.insert(id, ty);
        id
    }

    pub fn get_definition(&self, id: &DefId) -> Option<&Type> {
        self.definitions.get(id)
    }

    pub fn set_type(&mut self, id: NodeId, ty: Type) {
        self.nodes.insert(id, ty);
    }

    pub fn type_of(&self, id: &NodeId) -> Option<&Type> {
        self.nodes.get(id)
    }
}

pub fn infer(expr: &Expr, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<Type> {
    let ty = match expr {
        Expr::Ident(ident) => {
            let name = ident.inner.as_str();
            let def = scope
                .get_var(name)
                .ok_or(TypeError::NotFound(ident.to_owned()))?;
            let ty = env
                .get_definition(def)
                .expect("collected type for definition");
            Ok(ty.to_owned())
        }

        Expr::Literal(Literal { kind, .. }) => match kind {
            LiteralKind::Str(_) => Ok(Type::Str),
            LiteralKind::Int(_) => Ok(Type::Int(IntKind::I32)), // TODO: infer the correct size
        },

        Expr::Block(block) => {
            let scope = &mut scope.child();
            check_nodes(&block.nodes, env, scope)?;

            if let Some(last_node) = block.nodes.last() {
                let return_ty = match last_node {
                    Node::Expr(expr) => env.type_of(&expr.id()).unwrap().clone(),
                    Node::Stmnt(Stmnt::Ret(Ret { val, .. })) => {
                        env.type_of(&val.id()).unwrap().clone()
                    }
                    _ => Type::None,
                };
                Ok(return_ty)
            } else {
                Ok(Type::None)
            }
        }

        Expr::BinOp(BinOp { lhs, op, rhs, .. }) => {
            let lhs_ty = infer(lhs.as_ref(), env, scope)?;
            let rhs_ty = infer(rhs.as_ref(), env, scope)?;

            match op.kind {
                OpKind::Eq | OpKind::Lt | OpKind::Gt => {
                    if lhs_ty != rhs_ty {
                        Err(TypeError::ComparisonMismatch {
                            src: scope.src.clone(),
                            lhs_span: lhs.span(),
                            lhs_ty,
                            rhs_span: rhs.span(),
                            rhs_ty,
                            help: None,
                        })
                    } else {
                        Ok(Type::Bool)
                    }
                }

                OpKind::And | OpKind::Or => {
                    if lhs_ty != Type::Bool {
                        todo!()
                    }
                    if rhs_ty != Type::Bool {
                        todo!()
                    }

                    Ok(Type::Bool)
                }

                _ => match (lhs_ty, rhs_ty) {
                    (Type::Int(lhs_kind), Type::Int(rhs_kind)) => match op.kind {
                        OpKind::Add | OpKind::Sub | OpKind::Mul | OpKind::Div => {
                            Ok(Type::Int(lhs_kind))
                        }
                        _ => todo!(),
                    },
                    _ => todo!(),
                },
            }
        }

        Expr::Prefix(PrefixExpr { op, rhs, .. }) => {
            let ty = infer(rhs, env, scope)?;
            match (&op.kind, ty) {
                (OpKind::Sub, Type::Int(kind)) => Ok(Type::Int(kind)),
                _ => todo!(),
            }
        }

        Expr::Call(CallExpr { func, params, .. }) => {
            let Type::Fn {
                is_extern,
                params: param_types,
                returns,
            } = infer(func, env, scope)?
            else {
                todo!("cannot call a non fn var");
            };

            for param in params {
                let ty = infer(param, env, scope)?;
                // TODO:
            }

            // TODO: check validity of params

            Ok(returns.as_ref().to_owned())
        }

        Expr::Index(IndexExpr { expr, .. }) => {
            if let Type::List((ty, _)) = infer(expr, env, scope)? {
                Ok(ty.as_ref().to_owned())
            } else {
                todo!("can only index for list types")
            }
        }

        Expr::IfElse(IfElse {
            id,
            condition,
            consequence,
            alternative,
            ..
        }) => {
            let condition_ty = infer(condition, env, scope)?;
            if condition_ty != Type::Bool {
                return Err(TypeError::InvalidType {
                    expected: Type::Bool,
                    actual: condition_ty,
                });
            }

            let block_scope = &mut scope.child();
            let consequence_ty = infer(&Expr::Block(consequence.to_owned()), env, block_scope)?;
            let alternative_ty = alternative
                .clone()
                .map(|alternative| infer(&Expr::Block(alternative), env, block_scope))
                .transpose()?;

            if let Some(alternative_ty) = alternative_ty
                && let Some(alternative) = alternative
                && alternative_ty != consequence_ty
            {
                return Err(TypeError::ComparisonMismatch {
                    src: scope.src.clone(),
                    lhs_span: consequence.span,
                    lhs_ty: consequence_ty,
                    rhs_span: alternative.span,
                    rhs_ty: alternative_ty,
                    help: None,
                });
            }

            Ok(consequence_ty)
        }

        Expr::List(List { items, .. }) => {
            todo!()
        }

        Expr::Constructor(Constructor { ident, fields, .. }) => {
            let def_id = scope
                .get_var(ident)
                .ok_or(TypeError::NotFound(ident.to_owned()))?;
            let ty = env
                .get_definition(def_id)
                .expect("constructor type to be defined");
            Ok(ty.to_owned())
        }

        Expr::Ref(expr) => Ok(Type::Ptr(infer(expr, env, scope)?.into())),

        Expr::RawIdent(_ident) => todo!("infer raw ident"),
    }?;

    env.set_type(expr.id(), ty.clone());

    Ok(ty)
}

pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match stmnt {
        Stmnt::Let(Let { ident, ty, val, .. }) => {
            let ty = if let Some(ty) = ty {
                Type::from(ty)
            } else {
                infer(val, env, scope)?
            };

            let def_id = env.define(ty);
            scope.set_var(ident, def_id);
        }

        Stmnt::Ret(Ret { val, .. }) => {
            infer(val, env, scope)?;
        }

        Stmnt::Use(Use { ident, .. }) => {}

        Stmnt::Fn(Fn {
            is_extern,
            ident,
            params,
            return_ty,
            body,
            ..
        }) => {
            let ty = Type::Fn {
                is_extern: *is_extern,
                params: params.iter().map(|(_, ty)| ty.into()).collect(),
                returns: Box::new(return_ty.into()),
            };
            let def_id = env.define(ty);
            scope.set_var(ident, def_id);

            let body_ty = if let Some(body) = body
                && !is_extern
            {
                let block = Expr::Block(body.to_owned()); // FIX: no need for this clone
                let scope = &mut scope.child();
                for (ident, ty) in params {
                    let def_id = env.define(ty.into());
                    scope.set_var(ident, def_id);
                }

                infer(&block, env, scope)?
            } else {
                Type::None
            };
        }

        Stmnt::StructDef(StructDef { ident, fields, .. }) => {
            let ty = Type::Struct {
                ident: ident.to_owned(),
                fields: fields
                    .iter()
                    .map(|(ident, ty)| (ident.to_owned(), ty.into()))
                    .collect(),
            };
            let def_id = env.define(ty);
            scope.set_var(ident, def_id);
        }

        Stmnt::Impl(Impl { ident, body, .. }) => todo!("check impl block"),
    }

    Ok(())
}

pub fn check_node(node: &Node, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    match node {
        Node::Expr(expr) => {
            let ty = infer(expr, env, scope)?;
            env.set_type(expr.id(), ty);
            Ok(())
        }
        Node::Stmnt(stmnt) => check_stmnt(stmnt, env, scope),
    }
}

pub fn check_nodes(nodes: &[Node], env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<()> {
    for node in nodes {
        check_node(node, env, scope)?;
    }
    Ok(())
}
