use crate::ast::*;

pub trait Visitor<T> {
    fn visit_module(&mut self, module: &Module) -> T;
    fn visit_item(&mut self, item: &Item) -> T;
    fn visit_fn(&mut self, func: &Fn) -> T;
    fn visit_struct_def(&mut self, struct_def: &StructDef) -> T;
    fn visit_impl(&mut self, imp: &Impl) -> T;
    fn visit_stmnt(&mut self, stmnt: &Stmnt) -> T;
    fn visit_let(&mut self, binding: &Let) -> T;
    fn visit_ret(&mut self, ret: &Ret) -> T;
    fn visit_expr(&mut self, expr: &Expr) -> T;
    fn visit_ident(&mut self, ident: &Ident) -> T;
    fn visit_lit(&mut self, literal: &Literal) -> T;
    fn visit_block(&mut self, block: &Block) -> T;
    fn visit_bin_op(&mut self, bin_op: &BinOp) -> T;
    fn visit_unary(&mut self, unary: &Unary) -> T;
    fn visit_call(&mut self, call: &Call) -> T;
    fn visit_index(&mut self, index: &Index) -> T;
    fn visit_if_else(&mut self, index: &IfElse) -> T;
    fn visit_list(&mut self, list: &List) -> T;
    fn visit_constructor(&mut self, constructor: &Constructor) -> T;
    fn visit_member_access(&mut self, member_access: &MemberAccess) -> T;
    fn visit_ref(&mut self, inner: &Expr) -> T;
    fn visit_assign(&mut self, assign: &Assign) -> T;
}
