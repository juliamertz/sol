use std::collections::HashMap;

use miette::Diagnostic;
use thiserror::Error;

use crate::lexer::source::{SourceInfo, Span};
use crate::mir::{Block, BlockId, Constant, Fn, Instruction, Operand, TempId, Terminator};
use crate::type_checker::{DefId, TypeEnv, TypeId};
use crate::{ast, hir};

#[derive(Debug, Default)]
pub(super) struct BlockBuilder {
    body: Vec<Instruction>,
    term: Option<Terminator>,
}

#[derive(Debug, Diagnostic, Error)]
pub enum BlockBuilderError {
    #[error("may not terminate the same block more than once")]
    AlreadyTerminated,
    #[error("block has not been terminated yet")]
    Unterminated,
}

impl BlockBuilder {
    pub fn push_instr(&mut self, instr: Instruction) -> &mut Self {
        self.body.push(instr);
        self
    }

    pub fn is_terminated(&self) -> bool {
        self.term.is_some()
    }

    pub fn terminate(&mut self, term: Terminator) -> Result<(), BlockBuilderError> {
        if self.is_terminated() {
            dbg!(&term, &self.term);

            Ok(())
            // Err(BlockBuilderError::AlreadyTerminated)
        } else {
            self.term = Some(term);
            Ok(())
        }
    }

    pub fn build(self) -> Result<Block, BlockBuilderError> {
        Ok(Block {
            body: self.body,
            term: self.term.ok_or(BlockBuilderError::Unterminated)?,
        })
    }
}

#[derive(Debug, Diagnostic, Error)]
pub enum BuilderError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    BlockBuilder(#[from] BlockBuilderError),
    #[error("local not found")]
    LocalNotFound {
        #[source_code]
        src: SourceInfo,

        #[label("this one")]
        span: Span,
    },
}

pub type Result<T, E = BuilderError> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct Builder<'tcx> {
    env: &'tcx TypeEnv,
    temp_idx: usize,
    temp_tys: Vec<TypeId>,
    block_idx: usize,
    blocks: Vec<BlockBuilder>,
    locals: HashMap<DefId, Operand>,
}

impl<'tcx> Builder<'tcx> {
    pub fn new(env: &'tcx TypeEnv) -> Builder<'tcx> {
        Self {
            env,
            temp_idx: 0,
            temp_tys: vec![],
            block_idx: 0,
            blocks: vec![],
            locals: HashMap::default(),
        }
    }

    pub(super) fn new_temp(&mut self, ty: TypeId) -> TempId {
        let id = TempId(self.temp_idx);
        self.temp_tys.push(ty);
        self.temp_idx += 1;
        id
    }

    pub(super) fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.block_idx);
        self.blocks.push(BlockBuilder::default());
        self.block_idx += 1;
        id
    }

    pub(super) fn get_block_mut(&mut self, BlockId(idx): &BlockId) -> &mut BlockBuilder {
        &mut self.blocks[*idx]
    }

    pub(super) fn define_local(&mut self, def_id: DefId, operand: Operand) {
        self.locals.insert(def_id, operand);
    }

    pub fn build(
        self,
        name: impl ToString,
        return_ty: TypeId,
        params: impl Iterator<Item = TypeId>,
    ) -> Result<Fn> {
        let name = name.to_string();
        let blocks = self
            .blocks
            .into_iter()
            .map(|b| b.build())
            .collect::<Result<Vec<_>, BlockBuilderError>>()?;
        let temps = self.temp_tys;
        let params = params.into_iter().collect();
        Ok(Fn {
            name,
            return_ty,
            params,
            temps,
            blocks,
        })
    }

    pub(super) fn lower_hir_block(
        &mut self,
        hir_block: &hir::Block<'_>,
        block: BlockId,
    ) -> Result<(Operand, BlockId)> {
        let mut last_val = None;
        let mut last_block = block;

        for stmnt in hir_block.nodes.iter() {
            if let hir::Stmnt::Expr(expr) = stmnt {
                let (val, block) = self.lower_expr(expr, last_block)?;
                last_val = Some(val);
                last_block = block;
            } else {
                last_block = self.lower_stmnt(stmnt, last_block)?;
            }
        }

        let val = last_val.unwrap_or(Operand::Constant(Constant::Unit));
        Ok((val, last_block))
    }

    pub(super) fn lower_stmnt(
        &mut self,
        stmnt: &hir::Stmnt<'_>,
        block: BlockId,
    ) -> Result<BlockId> {
        Ok(match stmnt {
            hir::Stmnt::Let(binding) => {
                let (val, block) = self.lower_expr(&binding.val, block)?;
                self.locals.insert(binding.def_id, val);
                block
            }
            hir::Stmnt::Ret(ret) => {
                let (val, block) = self.lower_expr(&ret.val, block)?;
                self.get_block_mut(&block)
                    .terminate(Terminator::Return(val))?;
                block
            }
            hir::Stmnt::Expr(expr) => {
                let (_val, block) = self.lower_expr(expr, block)?;
                block
            }
        })
    }

    pub(super) fn lower_expr(
        &mut self,
        expr: &hir::Expr<'_>,
        block: BlockId,
    ) -> Result<(Operand, BlockId)> {
        match expr {
            hir::Expr::BinOp(bin_op) => {
                let (lhs, block) = self.lower_expr(&bin_op.lhs, block)?;
                let (rhs, block) = self.lower_expr(&bin_op.rhs, block)?;
                let dest = self.new_temp(bin_op.ty);
                let op = bin_op.op;
                self.get_block_mut(&block)
                    .push_instr(Instruction::bin_op(dest, op.kind, lhs, rhs));

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::IfElse(if_else) => {
                let dest = self.new_temp(if_else.ty);
                let conseq_block = self.new_block();
                let alt_block = self.new_block();
                let join_block = self.new_block();

                let (cond, block) = self.lower_expr(&if_else.condition, block)?;
                self.get_block_mut(&block).terminate(Terminator::branch(
                    cond,
                    conseq_block,
                    alt_block,
                ))?;

                let (conseq_val, conseq_exit) =
                    self.lower_hir_block(&if_else.consequence, conseq_block)?;
                self.get_block_mut(&conseq_exit)
                    .push_instr(Instruction::copy(dest, conseq_val))
                    .terminate(Terminator::goto(join_block))?;

                let (alt_val, alt_exit) =
                    self.lower_hir_block(if_else.alternative.as_ref().unwrap(), alt_block)?;
                self.get_block_mut(&alt_exit)
                    .push_instr(Instruction::copy(dest, alt_val))
                    .terminate(Terminator::goto(join_block))?;

                Ok((Operand::Temporary(dest), join_block))
            }

            hir::Expr::Literal(literal) => Ok((
                Operand::Constant(match literal.kind {
                    ast::LiteralKind::Str(val) => Constant::Str(val.to_string()),
                    ast::LiteralKind::Int(val) => Constant::Int(*val),
                    ast::LiteralKind::Bool(val) => Constant::Bool(*val),
                }),
                block,
            )),

            hir::Expr::Unary(unary) => {
                let dest = self.new_temp(unary.ty);
                let op = &unary.op;
                let rhs = &unary.rhs;
                let (rhs, block) = self.lower_expr(rhs, block)?;

                self.get_block_mut(&block)
                    .push_instr(Instruction::unary_op(dest, op.kind, rhs));

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::Call(call) => {
                let dest = self.new_temp(call.ty);
                let (operands, block) = call.params.iter().try_fold(
                    (Vec::with_capacity(16), block),
                    |(mut acc, block), expr| {
                        let (val, block) = self.lower_expr(expr, block)?;
                        acc.push(val);
                        Ok::<_, BuilderError>((acc, block))
                    },
                )?;

                self.get_block_mut(&block).push_instr(Instruction::call(
                    dest,
                    call.def_id,
                    operands,
                ));

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::Block(hir_block) => self.lower_hir_block(hir_block, block),

            hir::Expr::Ident(ident) => {
                let val = self
                    .locals
                    .get(&ident.def_id)
                    .ok_or(BuilderError::LocalNotFound {
                        src: self.env.src.clone(),
                        span: *ident.span,
                    })?;
                Ok((val.clone(), block))
            }

            // TODO:
            _ => Ok((Operand::Constant(Constant::Unit), block)),
            // hir::Expr::Index(index) => todo!(),
            // hir::Expr::List(list) => todo!(),
            // hir::Expr::Constructor(constructor) => todo!(),
            // hir::Expr::MemberAccess(member_access) => todo!(),
            // hir::Expr::Ref(expr) => todo!(),
        }
    }
}
