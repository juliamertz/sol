use miette::{Context, Diagnostic, IntoDiagnostic, Report, Result, SourceSpan, miette};
use std::{collections::HashMap, vec};
use thiserror::Error;

use crate::ast::{self, Op};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    // TODO: We should remove this, but for now we can use it as a crutch
    // Or we move to Unkown and directly parse into this ast
    Any,
    Unit,
    Int,
    Bool,
    Str,
    List((Box<Type>, Option<usize>)),
    Ptr(Box<Type>),
    Fn {
        is_extern: bool, // TODO: i don't think it makes sense to store this info in the type.
        args: Vec<Type>,
        returns: Box<Type>,
    },
    Struct {
        ident: String,
        fields: Vec<(String, Type)>,
    },
    // TODO: include sourcespan so we can have nicer debug messages
    Var(String),
}

impl Type {
    fn list(ty: Type, size: Option<usize>) -> Self {
        Self::List((Box::new(ty), size))
    }

    fn ptr(ty: Type) -> Self {
        Self::Ptr(Box::new(ty))
    }

    fn is_concrete(&self) -> bool {
        !matches!(self, Self::Var(_))
    }
}

impl From<&ast::TypeExpr> for Type {
    fn from(value: &ast::TypeExpr) -> Self {
        match value {
            ast::TypeExpr::Int => Self::Int,
            ast::TypeExpr::Bool => Self::Bool,
            ast::TypeExpr::Str => Self::Str,
            ast::TypeExpr::List((ty, size)) => Self::list(Type::from(&(**ty)), *size),
            ast::TypeExpr::Fn {
                args,
                returns,
                is_extern,
            } => todo!(),
            ast::TypeExpr::Var(name) => Self::Var(name.clone()),
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
pub enum TypeError {
    #[error("type mismatch, expected: {0:?}, got: {1:?}")]
    TypeMismatch(Type, Type),

    #[error("no such variable: '{0}'")]
    UndefinedVariable(String),

    #[error("no such type: '{0}'")]
    UndefinedType(String),

    #[error("ambigous type")]
    AmbigiousType,

    #[error("not a function")]
    NotAFunction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Var,
    Fn,
    Field,
    Struct,
    Method,
    Param,
}

type SymbolId = u32;

#[derive(Debug, Clone)]
pub struct Symbol {
    pub id: SymbolId,
    pub kind: SymbolKind,
    pub name: String,
    pub ty: Type,
}

type Level = u32;

#[derive(Debug, Clone)]
pub struct Module {
    pub env: TypeEnv,
    pub symbols: Vec<Symbol>,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Expr(Expr),
    Stmnt(Stmnt),
}

#[derive(Debug, Clone)]
pub enum Expr {
    IntLit(i64),
    StrLit(String),
    Var {
        id: SymbolId,
        ty: Type,
    },
    BinOp {
        lhs: Box<Expr>,
        op: ast::Op,
        rhs: Box<Expr>,
        ty: Type,
    },
    Unary {
        op: ast::Op,
        rhs: Box<Expr>,
        ty: Type,
    },
    Block {
        nodes: Vec<Node>,
        ty: Type,
    },
    Call {
        id: SymbolId,
        params: Vec<Expr>,
        ty: Type,
    },
    Index {
        id: SymbolId,
        idx: usize,
        ty: Type,
    },
    IfElse {
        condition: Box<Expr>,
        consequence: Vec<Node>,
        alternative: Option<Vec<Node>>,
        ty: Type,
    },
    List(Vec<Expr>),
    Constructor {
        id: SymbolId,
        fields: Vec<(String, Expr)>,
    },
    Ref(Box<Expr>),
    Deref(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Stmnt {
    Let {
        id: SymbolId,
        val: Expr,
    },
    Fn {
        id: SymbolId,
        r#extern: bool,
        params: Vec<SymbolId>,
        body: Vec<Node>,
        returns: Type,
    },
    Ret {
        implicit: bool,
        val: Option<Expr>,
        ty: Type,
    },
    Use {
        path: Vec<String>,
    },
    Struct {
        id: SymbolId,
        impls: Vec<SymbolId>,
    },
}

#[derive(Debug, Default, Clone)]
pub struct TypeEnv {
    types: HashMap<String, SymbolId>,
    variables: HashMap<String, SymbolId>,
}

#[derive(Default)]
pub struct HirBuilder {
    symbols: Vec<Symbol>,
}

impl HirBuilder {
    fn new_symbol(&mut self, name: impl ToString, ty: Type, kind: SymbolKind) -> &Symbol {
        let id = self.symbols.len();
        self.symbols.push(Symbol {
            id: id.try_into().unwrap(),
            kind,
            name: name.to_string(),
            ty,
        });
        unsafe { self.symbols.get_unchecked(id) }
    }

    fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(id as usize)
    }

    fn get_var(&self, name: impl AsRef<str>, env: &TypeEnv) -> Result<&Symbol> {
        env.variables
            .get(name.as_ref())
            .and_then(|id| self.get_symbol(*id))
            .ok_or(TypeError::UndefinedVariable(name.as_ref().to_string()).into())
    }

    fn get_type(&self, name: impl AsRef<str>, env: &TypeEnv) -> Result<&Symbol> {
        env.types
            .get(name.as_ref())
            .and_then(|id| self.get_symbol(*id))
            .ok_or(TypeError::UndefinedType(name.as_ref().to_string()).into())
    }

    fn infer(&self, node: &ast::Node, env: &mut TypeEnv) -> Result<Type> {
        match node {
            ast::Node::Expr(expr) => self.infer_expr(expr, env),
            ast::Node::Stmnt(stmnt) => self.infer_stmnt(stmnt, env),
        }
    }

    fn infer_block(&self, block: &ast::Block, env: &mut TypeEnv) -> Result<Type> {
        let return_types: Vec<Type> = block
            .nodes
            .iter()
            .filter_map(|node| match node {
                ast::Node::Stmnt(ast::Stmnt::Ret(ast::Ret { val })) => {
                    self.infer_expr(val, env).ok()
                }
                _ => None,
            })
            .collect();

        let first = return_types.first().unwrap_or(&Type::Unit);
        if !return_types.iter().all(|ty| ty == first) {
            return Err(TypeError::AmbigiousType)
                .into_diagnostic()
                .wrap_err("multiple return types found in block");
        }

        Ok(first.clone())
    }

    fn infer_expr(&self, expr: &ast::Expr, env: &mut TypeEnv) -> Result<Type> {
        match expr {
            ast::Expr::IntLit(_) => Ok(Type::Int),
            ast::Expr::StrLit(_) => Ok(Type::Str),

            ast::Expr::Block(block) => self.infer_block(block, env),

            ast::Expr::Prefix(prefix_expr) => self.infer_expr(&prefix_expr.rhs, env),

            ast::Expr::BinOp(binop) => {
                let lhs = self.infer_expr(&binop.lhs, env)?;
                let rhs = self.infer_expr(&binop.rhs, env)?;
                // if lhs != rhs {
                //     dbg!(&lhs, &binop.op, &rhs);
                //     return Err(TypeError::TypeMismatch(lhs, rhs))
                //         .into_diagnostic()
                //         .wrap_err("binop type mismatch");
                // }

                // TODO: these errors should use the correct side
                match binop.op {
                    Op::Eq => {
                        if lhs != rhs {
                            Err(TypeError::TypeMismatch(Type::Bool, lhs))
                                .into_diagnostic()
                                .wrap_err("in eq op")
                        } else {
                            Ok(Type::Bool)
                        }
                    }

                    Op::And | Op::Or => match (&lhs, &rhs) {
                        (Type::Bool, Type::Bool) => Ok(Type::Bool),
                        _ => Err(TypeError::TypeMismatch(Type::Bool, lhs))
                            .into_diagnostic()
                            .wrap_err("Non bool value in and | or op"),
                    },

                    Op::Add | Op::Sub | Op::Mul | Op::Div => Ok(lhs),

                    Op::Lt | Op::Gt => match (&lhs, &rhs) {
                        (Type::Int, Type::Int) => Ok(Type::Bool),
                        _ => Err(TypeError::TypeMismatch(Type::Int, lhs))
                            .into_diagnostic()
                            .wrap_err("Non integer value in lt or gt op"),
                    },

                    Op::Chain => todo!(),
                }
            }

            ast::Expr::List(list) => {
                let mut items = list.items.iter();
                let expected_ty = items
                    .next()
                    .map(|item| self.infer_expr(item, env))
                    .unwrap_or(Ok(Type::Any))?; // TODO: how do we handle inferring empty lists?

                for item in items {
                    let ty = self.infer_expr(item, env)?;
                    if ty != expected_ty {
                        return Err(TypeError::TypeMismatch(expected_ty, ty))
                            .into_diagnostic()
                            .wrap_err("List type mismatch");
                    }
                }

                Ok(Type::list(expected_ty, None)) // TODO: fixed size lists
            }

            ast::Expr::Ident(name) => self.get_var(name, env).map(|sym| sym.ty.clone()),

            ast::Expr::Constructor(constructor) => self
                .get_type(&constructor.name, env)
                .map(|sym| sym.ty.clone()),

            ast::Expr::Call(call_expr) => match self.infer_expr(&call_expr.func, env)? {
                Type::Fn { returns, .. } => Ok(*returns),
                _ => todo!(),
            },

            ast::Expr::Ref(inner) => Ok(Type::ptr(self.infer_expr(inner, env)?)),

            ast::Expr::Index(expr) => {
                let Type::List((inner_ty, _)) = self.infer_expr(&expr.val, env)? else {
                    todo!("index val not a list");
                };

                Ok(*inner_ty)
            }

            ast::Expr::IfElse(if_else) => {
                let Type::Bool = self.infer_expr(&if_else.condition, env)? else {
                    panic!("condition is not a bool");
                };

                let return_ty = self.infer_block(&if_else.consequence, env)?;

                if let Some(ref alternative) = if_else.alternative {
                    let other_ty = self.infer_block(alternative, env)?;
                    if other_ty != return_ty {
                        panic!("at the disco");
                    }
                }

                Ok(return_ty)
            }

            // | ast::Expr::RawIdent(_) => unimplemented!(),
            _ => todo!("impl {expr:?}"),
        }
    }

    fn infer_stmnt(&self, stmnt: &ast::Stmnt, env: &mut TypeEnv) -> Result<Type> {
        match stmnt {
            ast::Stmnt::Let(binding) => binding
                .ty
                .as_ref()
                .map(Type::from)
                .map(Ok)
                .unwrap_or(self.infer_expr(&binding.val, env)),

            ast::Stmnt::Fn(binding) => {
                let mut args: Vec<Type> = vec![];
                for (_, ty) in binding.params.iter() {
                    args.push((ty).into());
                }

                let ty = Type::Fn {
                    args,
                    is_extern: binding.is_extern,
                    returns: Box::new((&binding.return_ty).into()),
                };

                Ok(ty)
            }

            ast::Stmnt::StructDef(def) => Ok(Type::Struct {
                ident: def.ident.clone(),
                fields: def
                    .fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), ty.into()))
                    .collect(),
            }),

            _ => Ok(Type::Any),
        }
    }

    pub fn lower(&mut self, ast: Vec<ast::Node>, env: &mut TypeEnv) -> Result<Module> {
        dbg!(&env);
        let nodes = self.lower_nodes(ast, env);
        dbg!(&env);
        Ok(Module {
            env: env.clone(),
            symbols: self.symbols.clone(),
            nodes: nodes?,
        })
    }

    pub fn lower_node(&mut self, node: ast::Node, env: &mut TypeEnv) -> Result<Node> {
        match node {
            ast::Node::Expr(expr) => Ok(Node::Expr(self.lower_expr(expr, env)?)),
            ast::Node::Stmnt(stmnt) => Ok(Node::Stmnt(self.lower_stmnt(stmnt, env)?)),
        }
    }

    pub fn lower_nodes(&mut self, nodes: Vec<ast::Node>, env: &mut TypeEnv) -> Result<Vec<Node>> {
        nodes
            .into_iter()
            .map(|node| self.lower_node(node, env))
            .try_fold(vec![], |mut acc, res| {
                acc.push(res?);
                Ok(acc)
            })
    }

    pub fn lower_expr_list(
        &mut self,
        exprs: Vec<ast::Expr>,
        env: &mut TypeEnv,
    ) -> Result<Vec<Expr>> {
        exprs
            .into_iter()
            .map(|param| self.lower_expr(param, env))
            .try_fold(vec![], |mut acc, res| {
                acc.push(res?);
                Ok(acc)
            })
    }

    pub fn lower_expr(&mut self, expr: ast::Expr, env: &mut TypeEnv) -> Result<Expr> {
        let ty = self.infer_expr(&expr, env)?;

        Ok(match expr {
            ast::Expr::IntLit(val) => Expr::IntLit(val),
            ast::Expr::StrLit(val) => Expr::StrLit(val),

            ast::Expr::Block(ref block) => Expr::Block {
                ty: self.infer_expr(&expr, env)?,
                nodes: block
                    .nodes
                    .clone()
                    .into_iter()
                    .map(|node| self.lower_node(node, env).unwrap())
                    .collect(),
            },

            ast::Expr::Call(call_expr) => {
                let ast::Expr::Ident(ident) = *call_expr.func else {
                    panic!("todo: non ident func / method");
                };

                let sym = self.get_var(ident, env)?;
                if sym.kind != SymbolKind::Fn {
                    return Err(TypeError::NotAFunction)
                        .into_diagnostic()
                        .wrap_err("tried to call a variable which is not a function");
                }

                Expr::Call {
                    id: sym.id,
                    ty: sym.ty.clone(),
                    params: self.lower_expr_list(call_expr.params, env)?,
                }
            }

            ast::Expr::Ident(ident) => {
                let sym = self.get_var(ident, env)?;
                Expr::Var {
                    id: sym.id,
                    ty: sym.ty.clone(),
                }
            }

            ast::Expr::Constructor(constructor) => {
                let sym = self.get_var(&constructor.name, env)?;
                if sym.kind != SymbolKind::Struct {
                    panic!("constructor must be a struct");
                }

                Expr::Constructor {
                    id: sym.id,
                    fields: constructor
                        .fields
                        .into_iter()
                        .map(|(name, val)| (name, self.lower_expr(val, env).unwrap()))
                        .collect(),
                }
            }

            ast::Expr::BinOp(binop) => Expr::BinOp {
                lhs: self.lower_expr(*binop.lhs, env)?.into(),
                op: binop.op,
                rhs: self.lower_expr(*binop.rhs, env)?.into(),
                ty,
            },

            ast::Expr::RawIdent(_) => todo!(),

            ast::Expr::Prefix(prefix_expr) => todo!(),

            ast::Expr::Index(index_expr) => todo!(),

            ast::Expr::IfElse(if_else) => Expr::IfElse {
                condition: self.lower_expr(*if_else.condition, env)?.into(),
                consequence: self.lower_nodes(if_else.consequence.nodes, env)?,
                alternative: None, // TODO: else
                ty,
            },

            ast::Expr::List(list) => {
                let mut items = vec![];
                for expr in list.items {
                    items.push(self.lower_expr(expr, env)?);
                }

                Expr::List(items)
            }

            ast::Expr::Ref(expr) => todo!(),
        })
    }

    pub fn lower_stmnt(&mut self, stmnt: ast::Stmnt, env: &mut TypeEnv) -> Result<Stmnt> {
        let ty = self.infer_stmnt(&stmnt, env)?;
        let stmnt = match stmnt {
            ast::Stmnt::Fn(func) => {
                let sym = self.new_symbol(func.name.clone(), ty, SymbolKind::Fn);
                let Type::Fn { returns, .. } = sym.ty.clone() else {
                    panic!("not a function!?!?!?!?!?!?");
                };
                let func_id = sym.id;

                env.variables.insert(func.name, func_id);
                let mut func_env = env.clone();

                let mut params = vec![];
                for (name, ref ty) in func.params {
                    let sym = self.new_symbol(name.clone(), ty.into(), SymbolKind::Var);
                    params.push(sym.id);
                    func_env.variables.insert(name, sym.id);
                }

                let mut body = vec![];
                if !func.is_extern {
                    for node in func.body.expect("function to have a body!").nodes {
                        let node = self.lower_node(node, &mut func_env)?;
                        body.push(node);
                    }
                }

                Stmnt::Fn {
                    id: func_id,
                    r#extern: func.is_extern,
                    params,
                    body,
                    returns: *returns,
                }
            }

            ast::Stmnt::Let(binding) => {
                let sym = self.new_symbol(binding.name.clone(), ty.clone(), SymbolKind::Var);
                env.variables.insert(binding.name, sym.id);
                Stmnt::Let {
                    id: sym.id,
                    val: self.lower_expr(binding.val, env)?,
                }
            }

            ast::Stmnt::Ret(ret) => Stmnt::Ret {
                implicit: false,
                val: self.lower_expr(ret.val, env)?.into(),
                ty,
            },
            ast::Stmnt::Use(import) => Stmnt::Use {
                path: vec![import.ident], // TODO: proper paths
            },
            ast::Stmnt::StructDef(struct_def) => todo!(),
            ast::Stmnt::Impl(_) => todo!(),
        };

        Ok(stmnt)
    }
}
