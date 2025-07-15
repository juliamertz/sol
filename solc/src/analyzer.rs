// use miette::{Diagnostic, Result};
use solc_macros::Id;
use std::{collections::HashMap, hash::Hash};

use crate::ast::{BinOp, Expr, Ident, LiteralKind, Node, NodeId, OpKind, Stmnt};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("variable not found in scope: {0}")]
    NotFound(Ident),
    #[error("type mismatch: {lhs:?} {rhs:?}")]
    TypeMismatch { lhs: Type, rhs: Type },
}

#[derive(Id, Debug)]
pub struct DefId(u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    None,
    Int,
    Bool,
    Str,
    List((Box<Type>, Option<usize>)),
    Ptr(Box<Type>),
    Fn {
        is_extern: bool,
        args: Vec<Type>,
        returns: Box<Type>,
    },
    Struct {
        ident: Ident,
        fields: Vec<(Ident, Type)>,
    },
    Var(Ident),
}

pub struct Scope<'a> {
    parent: Option<&'a Scope<'a>>,
    variables: HashMap<String, DefId>,
}

impl Scope<'_> {
    pub fn set_var(&mut self, name: impl ToString, def_id: DefId) {
        self.variables.insert(name.to_string(), def_id);
    }

    pub fn get_var(&self, name: impl AsRef<str>) -> Option<&DefId> {
        self.variables
            .get(name.as_ref())
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get_var(name)))
    }
}

pub struct TypeEnv {
    nodes: HashMap<NodeId, Type>,
    definitions: HashMap<DefId, Type>,
}

impl TypeEnv {
    pub fn define(&mut self, id: DefId, ty: Type) {
        self.definitions.insert(id, ty);
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

pub fn infer(expr: &Expr, env: &mut TypeEnv, scope: &mut Scope<'_>) -> Result<Type, Error> {
    match expr {
        Expr::Ident(ident) => {
            let name = ident.inner.as_str();
            let def = scope
                .get_var(name)
                .ok_or(Error::NotFound(ident.to_owned()))?;
            let ty = env
                .get_definition(def)
                .expect("collected type for definition");
            Ok(ty.to_owned())
        }
        Expr::Literal(literal) => Ok(match literal.kind {
            LiteralKind::Str(_) => Type::Str,
            LiteralKind::Int(_) => Type::Int,
        }),
        Expr::Block(block) => todo!(),
        Expr::BinOp(BinOp { lhs, op, rhs, .. }) => {
            let lhs = infer(lhs.as_ref(), env, scope)?;
            let rhs = infer(rhs.as_ref(), env, scope)?;

            if let OpKind::Eq | OpKind::Lt | OpKind::Gt = op.kind {
                if lhs != rhs {
                    return Err(Error::TypeMismatch { lhs, rhs });
                } else {
                    return Ok(Type::Bool);
                }
            };

            // TODO:
            // OpKind::And
            // OpKind::Or

            match (lhs, rhs) {
                (Type::Int, Type::Int) => match op.kind {
                    OpKind::Add | OpKind::Sub | OpKind::Mul | OpKind::Div => Ok(Type::Int),
                    _ => todo!(),
                },
                _ => todo!(),
            }
        }
        Expr::Prefix(prefix_expr) => todo!("infer prefix expr"),
        Expr::Call(call_expr) => todo!("infer call expr"),
        Expr::Index(index_expr) => todo!("infer index expr"),
        Expr::IfElse(_) => todo!("infer if else"),
        Expr::List(list) => todo!("infer list"),
        Expr::Constructor(constructor) => todo!("infer constructor"),
        Expr::Ref(expr) => infer(expr, env, scope),
        Expr::RawIdent(ident) => todo!("infer raw ident"),
    }
}

// impl From<&ast::TypeExpr> for Type {
//     fn from(value: &ast::TypeExpr) -> Self {
//         match value {
//             ast::TypeExpr::Int => Self::Int,
//             ast::TypeExpr::Bool => Self::Bool,
//             ast::TypeExpr::Str => Self::Str,
//             ast::TypeExpr::List { inner, size } => {
//                 let unboxed = Type::from(&(**inner));
//                 Self::list(unboxed, *size)
//             }
//             ast::TypeExpr::Fn {
//                 args,
//                 returns,
//                 is_extern,
//             } => todo!(),
//             ast::TypeExpr::Var(name) => Self::Var(name.clone()),
//         }
//     }
// }

// #[derive(Error, Diagnostic, Debug)]
// pub enum AnalyzeError {
//     #[error("Type mismatch between {lhs:?} and {rhs:?}")]
//     TypeMismatch { lhs: Type, rhs: Type },
//
//     #[error("No such variable: '{name}'")]
//     UndefinedVariable { name: String },
// }
//
// #[derive(Debug, Clone)]
// pub struct TypeInfo {
//     definitions: HashMap<String, Type>,
//     types: HashMap<NodeId, Type>,
// }
//
// impl TypeInfo {
//     pub fn new() -> Self {
//         Self {
//             definitions: HashMap::new(),
//             types: HashMap::new(),
//         }
//     }
//
//     pub fn type_of(&self, id: &NodeId) -> Option<&Type> {
//         self.types.get(id)
//     }
//
//     pub fn bind(&mut self, name: impl ToString, ty: Type) {
//         self.definitions.insert(name.to_string(), ty);
//     }
//
//     pub fn get_def(&self, name: impl AsRef<str>) -> Option<&Type> {
//         self.definitions.get(name.as_ref())
//     }
//
//     pub fn get_def_mut(&mut self, name: impl AsRef<str>) -> Option<&mut Type> {
//         self.definitions.get_mut(name.as_ref())
//     }
// }
//
// pub struct Analyzer;
//
// impl Analyzer {
//     /// put new declarations in the typeenv env
//     /// and return a list of items that should be pre-defined such as lists or strings
//     pub fn collect_declarations(nodes: &[Node], env: &mut TypeInfo) -> Result<Vec<(String, Type)>> {
//         // There must be a cleaner way to do this.
//         // TDO: cleanup
//         let mut predefine = vec![];
//
//         for node in nodes {
//             let def = match node {
//                 Node::Stmnt(Stmnt::Let(binding)) => {
//                     let ty = match binding.ty {
//                         Some(ref ty) => ty.into(),
//                         // TDO: check binding type
//                         None => Self::check_expr(&binding.val, env).unwrap(),
//                     };
//
//                     // TDO: this is a bit ugly
//                     if matches!(ty, Type::List(_)) {
//                         predefine.push((binding.ident.clone(), ty.clone()))
//                     }
//
//                     Some((&binding.ident, ty))
//                 }
//
//                 Node::Stmnt(Stmnt::Fn(binding)) => {
//                     let mut args = vec![];
//                     for (_, ty) in binding.params.iter() {
//                         args.push(ty.into());
//                     }
//
//                     let ty = Type::Fn {
//                         args,
//                         is_extern: binding.is_extern,
//                         returns: Box::new((&binding.return_ty).into()),
//                     };
//
//                     Some((&binding.name, ty))
//                 }
//
//                 Node::Stmnt(Stmnt::StructDef(def)) => {
//                     let ty = Type::Struct {
//                         ident: def.ident.clone(),
//                         fields: def
//                             .fields
//                             .iter()
//                             .map(|(ident, ty)| (ident.clone(), ty.into()))
//                             .collect(),
//                     };
//
//                     Some((&def.ident, ty))
//                 }
//
//                 _ => None,
//             };
//
//             let Some((name, ty)) = def else {
//                 continue;
//             };
//
//             env.bind(name, ty);
//         }
//         Ok(predefine)
//     }
//
//     pub fn _check_node(node: &Node, env: &mut TypeInfo) -> Result<Type> {
//         match node {
//             Node::Expr(expr) => Self::check_expr(expr, env),
//             Node::Stmnt(stmnt) => Self::check_stmnt(stmnt, env),
//         }
//     }
//
//     pub fn check_expr(expr: &Expr, env: &mut TypeInfo) -> Result<Type> {
//         match expr {
//             Expr::IntLit(_) => Ok(Type::Int),
//
//             Expr::Block(block) => {
//                 todo!("check block expressions")
//             }
//
//             Expr::StrLit(_) => Ok(Type::Str),
//
//             Expr::Prefix(prefix_expr) => {
//                 todo!();
//             }
//
//             Expr::BinOp(binop) => {
//                 let lhs = Self::check_expr(&binop.lhs, env)?;
//                 let rhs = Self::check_expr(&binop.rhs, env)?;
//                 if lhs != rhs {
//                     return Err(AnalyzeError::TypeMismatch { lhs, rhs }.into());
//                 }
//                 Ok(lhs)
//             }
//
//             Expr::List(list) => {
//                 let mut items = list.items.iter();
//                 let Some(first) = items.next() else {
//                     return Ok(Type::Any);
//                 };
//                 let expected_ty = Self::check_expr(first, env)?;
//
//                 for item in items {
//                     let ty = Self::check_expr(item, env)?;
//                     if ty != expected_ty {
//                         // TDO: add miette context that this is a list
//                         return Err(AnalyzeError::TypeMismatch {
//                             lhs: ty,
//                             rhs: expected_ty,
//                         }
//                         .into());
//                     }
//                 }
//
//                 Ok(Type::list(expected_ty, None))
//             }
//
//             Expr::Ident(name) => env
//                 .get_def(name)
//                 .cloned()
//                 .ok_or_else(|| AnalyzeError::UndefinedVariable { name: name.clone() }.into()),
//
//             Expr::Call(call_expr) => match Self::check_expr(&call_expr.func, env)? {
//                 Type::Fn { returns, .. } => Ok(*returns),
//                 _ => todo!(),
//             },
//
//             Expr::Constructor(constructor) => {
//                 // TDO: get type from type-env
//                 // let a= Checked::Known(Type::Struct { ident: constructor.ident, fields: () })
//
//                 let Some(ty) = env.get_def(&constructor.name) else {
//                     todo!("nice error message when struct does not exist {constructor:?}")
//                 };
//
//                 // TDO: i don't like having to clone here
//                 Ok(ty.clone())
//             }
//
//             Expr::Ref(inner) => Ok(Type::Ptr(Box::new(Self::check_expr(inner, env)?))),
//
//             Expr::Index(expr) => {
//                 let Type::List((inner_ty, _size)) = Self::check_expr(&expr.val, env)? else {
//                     todo!("index val not a list");
//                 };
//                 Ok(*inner_ty)
//             }
//
//             Expr::IfElse(_) | Expr::RawIdent(_) => unimplemented!(),
//         }
//     }
//
//     pub fn check_stmnt(stmnt: &Stmnt, env: &mut TypeInfo) -> Result<Type> {
//         match stmnt {
//             Stmnt::Let(binding) => {
//                 let value_ty = Analyzer::check_expr(&binding.val, env)?;
//
//                 match env.get_def_mut(&binding.ident) {
//                     Some(ty) if !ty.is_concrete() => {
//                         *ty = value_ty;
//                     }
//
//                     Some(known) => {
//                         // FX: incorrect
//                         if *known != value_ty {
//                             return Err(AnalyzeError::TypeMismatch {
//                                 lhs: known.clone(),
//                                 rhs: value_ty,
//                             }
//                             .into());
//                         }
//                     }
//
//                     _ => todo!(),
//                 }
//
//                 // let Some(ref expr) = binding.val else {
//                 //     return Ok(Type::Var(binding.name.clone()));
//                 // };
//                 // TDO: use type instead of inferring and then check it
//
//                 let ty = Self::check_expr(&binding.val, env).unwrap();
//                 env.bind(&binding.ident, ty.clone());
//                 Ok(ty)
//             }
//
//             Stmnt::Fn(binding) => {
//                 let mut args: Vec<Type> = vec![];
//                 for (_, ty) in binding.params.iter() {
//                     args.push((ty).into());
//                 }
//
//                 let ty = Type::Fn {
//                     args,
//                     is_extern: binding.is_extern,
//                     returns: Box::new((&binding.return_ty).into()),
//                 };
//
//                 Ok(ty)
//             }
//
//             Stmnt::StructDef(def) => Ok(Type::Struct {
//                 ident: def.ident.clone(),
//                 fields: def
//                     .fields
//                     .iter()
//                     .map(|(name, ty)| (name.clone(), ty.into()))
//                     .collect(),
//             }),
//
//             // Stmnt::Use(stmnt) => {
//             //     todo!();
//             // }
//
//             Stmnt::Use(_) | Stmnt::Ret(_) | Stmnt::Impl(_) => Ok(Type::Any),
//         }
//     }
// }
