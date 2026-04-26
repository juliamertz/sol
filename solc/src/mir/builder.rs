use std::collections::HashMap;
use std::mem;

use miette::Diagnostic;
use smallvec::SmallVec;
use thiserror::Error;

use crate::interner::Id;
use crate::lexer::source::{SourceInfo, Span};
use crate::mir::{
    Block, BlockId, Constant, Data, DataId, DataValue, Fn, Instruction, MirTy, Operand, TempId,
    Terminator,
};
use crate::type_checker::{DefId, FieldId, MemberResolution, TypeEnv, TypeId};
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

    pub fn push_instr_opt(&mut self, instr: Option<Instruction>) -> &mut Self {
        if let Some(instr) = instr {
            self.body.push(instr);
        }
        self
    }

    pub fn is_terminated(&self) -> bool {
        self.term.is_some()
    }

    pub fn terminate(&mut self, term: Terminator) -> Result<(), BlockBuilderError> {
        if self.is_terminated() {
            // TODO:
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

#[derive(Debug, Clone, Copy)]
pub struct LoopCtx {
    enter_block: BlockId,
    join_block: BlockId,
    #[allow(dead_code)] // TODO: implement return values for loops
    dest: TempId,
}

#[derive(Debug, Default)]
pub struct LoopStack(SmallVec<[LoopCtx; 8]>);

impl LoopStack {
    pub fn enter(&mut self, enter_block: BlockId, join_block: BlockId, dest: TempId) {
        self.0.push(LoopCtx {
            enter_block,
            join_block,
            dest,
        });
    }

    pub fn leave(&mut self) -> Option<LoopCtx> {
        self.0.pop()
    }

    pub fn current(&mut self) -> Option<&LoopCtx> {
        self.0.last()
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
    temp_tys: Vec<MirTy>,
    blocks: Vec<BlockBuilder>,
    locals: HashMap<DefId, Operand>,
    sret_dest: Option<TempId>,
    return_dest: Option<TempId>,
    loop_stack: LoopStack,
    data: Vec<Data>,
}

impl<'tcx> Builder<'tcx> {
    pub fn new(env: &'tcx TypeEnv) -> Builder<'tcx> {
        Self {
            env,
            temp_idx: 0,
            temp_tys: vec![],
            blocks: vec![],
            locals: HashMap::default(),
            sret_dest: None,
            return_dest: None,
            loop_stack: LoopStack::default(),
            data: Vec::default(),
        }
    }

    pub(super) fn new_temp(&mut self, ty: MirTy) -> TempId {
        let id = TempId::from(self.temp_idx);
        self.temp_tys.push(ty);
        self.temp_idx += 1;
        id
    }

    pub(super) fn new_block(&mut self) -> BlockId {
        let id = BlockId::from(self.blocks.len());
        self.blocks.push(BlockBuilder::default());
        id
    }

    pub(super) fn new_data(&mut self, value: DataValue) -> DataId {
        let id = DataId::from(self.data.len());
        self.data.push(Data { id, value });
        id
    }

    pub(super) fn get_block_mut(&mut self, BlockId(idx): &BlockId) -> &mut BlockBuilder {
        &mut self.blocks[*idx as usize]
    }

    pub(super) fn define_local(&mut self, def_id: DefId, operand: Operand) {
        self.locals.insert(def_id, operand);
    }

    pub(super) fn set_return_destination(&mut self, temp_id: TempId) {
        if self.return_dest.is_some() {
            todo!("error: cannot overwrite return ptr")
        }

        self.return_dest = Some(temp_id)
    }

    pub fn build(
        self,
        name: impl ToString,
        return_ty_id: MirTy,
        params: impl Iterator<Item = (TempId, MirTy)>,
    ) -> Result<(Fn, Vec<Data>)> {
        let name = name.to_string();
        let blocks = self
            .blocks
            .into_iter()
            .map(|b| b.build())
            .collect::<Result<Vec<_>, BlockBuilderError>>()?;
        let temps = self.temp_tys;
        let params = params.into_iter().collect();
        let func = Fn {
            name,
            return_ty: return_ty_id,
            params,
            temps,
            blocks,
        };
        Ok((func, self.data))
    }

    pub(super) fn lower_block(
        &mut self,
        block: &hir::Block<'_>,
        mut block_id: BlockId,
    ) -> Result<(Operand, BlockId)> {
        for stmnt in block.nodes.iter() {
            block_id = self.lower_stmnt(stmnt, block_id)?;
        }

        let (val, block) = block
            .returning
            .as_ref()
            .map(|expr| {
                self.sret_dest = mem::take(&mut self.return_dest);
                self.lower_expr(expr, block_id)
            })
            .transpose()?
            .unwrap_or((Operand::unit(), block_id));

        Ok((val, block))
    }

    pub(super) fn lower_stmnt(
        &mut self,
        stmnt: &hir::Stmnt<'_>,
        block: BlockId,
    ) -> Result<BlockId> {
        Ok(match stmnt {
            hir::Stmnt::Let(binding) => {
                let (val, block) = self.lower_expr(&binding.val, block)?;

                if binding.mutable {
                    let ty = MirTy::new(binding.ty);
                    let dest = self.new_temp(ty);
                    self.locals.insert(binding.def_id, Operand::Temporary(dest));
                    self.get_block_mut(&block)
                        .push_instr(Instruction::Alloc { dest, ty, count: 1 })
                        .push_instr(Instruction::Store { addr: dest, val });
                } else {
                    self.locals.insert(binding.def_id, val);
                }

                block
            }
            hir::Stmnt::Ret(ret) => {
                let (val, block) = self.lower_expr(&ret.val, block)?;
                self.sret_dest = mem::take(&mut self.return_dest);

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
                let dest = self.new_temp(MirTy::new(bin_op.ty));
                let op = bin_op.op;
                self.get_block_mut(&block)
                    .push_instr(Instruction::bin_op(dest, op.kind, lhs, rhs));

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::IfElse(if_else) => {
                let dest = self.new_temp(MirTy::new(if_else.ty));
                let conseq_block = self.new_block();
                let alt_block = if_else.alternative.as_ref().map(|_| self.new_block());
                let join_block = self.new_block();

                let (cond, block) = self.lower_expr(&if_else.condition, block)?;
                self.get_block_mut(&block).terminate(Terminator::branch(
                    cond,
                    conseq_block,
                    alt_block.unwrap_or(join_block),
                ))?;

                let (conseq_val, conseq_exit) =
                    self.lower_block(&if_else.consequence, conseq_block)?;
                self.get_block_mut(&conseq_exit)
                    .push_instr_opt(Instruction::copy_non_unit(dest, conseq_val))
                    .terminate(Terminator::goto(join_block))?;

                if let Some(alt_block) = alt_block
                    && let Some((alt_val, alt_exit)) = if_else
                        .alternative
                        .as_ref()
                        .map(|block| self.lower_block(block, alt_block))
                        .transpose()?
                {
                    self.get_block_mut(&alt_exit)
                        .push_instr_opt(Instruction::copy_non_unit(dest, alt_val))
                        .terminate(Terminator::goto(join_block))?;
                }

                Ok((Operand::Temporary(dest), join_block))
            }

            hir::Expr::Literal(literal) => Ok((
                match literal.kind {
                    ast::LiteralKind::Int(val) => {
                        Operand::Constant(Constant::Int(*val, MirTy::new(literal.ty)))
                    }
                    ast::LiteralKind::Float(val) => {
                        Operand::Constant(Constant::Float(*val, MirTy::new(literal.ty)))
                    }
                    ast::LiteralKind::Bool(val) => Operand::Constant(Constant::Bool(*val)),
                    ast::LiteralKind::Str(val) => {
                        let data_id = self.new_data(DataValue::String(val.to_string()));
                        Operand::Data(data_id)
                    }
                },
                block,
            )),

            hir::Expr::Unary(unary) => {
                let dest = self.new_temp(MirTy::new(unary.ty));
                let op = &unary.op;
                let rhs = &unary.rhs;
                let (rhs, block) = self.lower_expr(rhs, block)?;

                self.get_block_mut(&block)
                    .push_instr(Instruction::unary_op(dest, op.kind, rhs));

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::Call(call) => {
                let return_ty_id = call.ty;
                let return_ty = self.env.types.get(&return_ty_id);
                let must_allocate = return_ty.must_allocate();
                let return_ty = if must_allocate {
                    MirTy::new_ptr(return_ty_id)
                } else {
                    MirTy::new(return_ty_id)
                };
                let dest = self.new_temp(return_ty);

                // to prevent returning dangling pointers we must preallocate any non-basic types
                // and pass that address to the function we're calling
                let mut acc = Vec::with_capacity(16);
                if must_allocate {
                    self.get_block_mut(&block).push_instr(Instruction::Alloc {
                        dest,
                        ty: return_ty,
                        count: 1,
                    });
                    acc.push(Operand::Temporary(dest));
                }

                let (operands, block) =
                    call.params
                        .iter()
                        .try_fold((acc, block), |(mut acc, block), expr| {
                            let (val, block) = self.lower_expr(expr, block)?;
                            acc.push(val);
                            Ok::<_, BuilderError>((acc, block))
                        })?;

                self.get_block_mut(&block).push_instr(Instruction::call(
                    if must_allocate { None } else { Some(dest) },
                    call.def_id,
                    operands,
                ));

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::Block(hir_block) => self.lower_block(hir_block, block),

            hir::Expr::Ident(ident) => {
                let val = self
                    .locals
                    .get(&ident.def_id)
                    .ok_or(BuilderError::LocalNotFound {
                        src: self.env.src.clone(),
                        span: *ident.span,
                    })?;

                if ident.mutability.is_mutable() {
                    let addr = val.as_temp().copied().expect("value to be a local"); // TODO: error handling
                    let dest = self.new_temp(MirTy::new(ident.ty));
                    self.get_block_mut(&block)
                        .push_instr(Instruction::Load { dest, addr });
                    Ok((Operand::Temporary(dest), block))
                } else {
                    Ok((val.clone(), block))
                }
            }

            hir::Expr::List(list) => {
                let dest = self.new_temp(MirTy::new(list.ty));
                self.get_block_mut(&block).push_instr(Instruction::Alloc {
                    dest,
                    ty: MirTy::new_ptr(list.ty),
                    count: list.size,
                });

                list.items
                    .iter()
                    .enumerate()
                    .try_fold(block, |block, (idx, expr)| {
                        let (val, block) = self.lower_expr(expr, block)?;
                        let ptr_dest = self.new_temp(MirTy::new_ptr(list.ty));

                        self.get_block_mut(&block)
                            .push_instr(Instruction::IndexPtr {
                                dest: ptr_dest,
                                base: Operand::Temporary(dest),
                                index: Operand::Constant(Constant::Int(
                                    idx as i128,
                                    MirTy::new(TypeId::I64),
                                )),
                                elem_ty: MirTy::new(list.ty),
                            })
                            .push_instr(Instruction::Store {
                                addr: ptr_dest,
                                val,
                            });

                        Ok::<_, BuilderError>(block)
                    })?;

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::Index(index) => {
                let dest = self.new_temp(MirTy::new(index.ty));
                let ptr_dest = self.new_temp(MirTy::new_ptr(index.ty));
                let (base_val, block) = self.lower_expr(&index.expr, block)?;
                let (index_val, block) = self.lower_expr(&index.idx, block)?;
                let elem_ty = MirTy::new(index.expr.type_id().to_owned());

                self.get_block_mut(&block)
                    .push_instr(Instruction::IndexPtr {
                        dest: ptr_dest,
                        base: base_val,
                        index: index_val,
                        elem_ty,
                    })
                    .push_instr(Instruction::Load {
                        dest,
                        addr: ptr_dest,
                    });

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::Assign(assign) => {
                // TODO: would be nice to refactor this to places/projections later on
                match assign.lhs.as_ref() {
                    hir::Expr::Ident(ident) => {
                        if ident.mutability.is_immutable() {
                            todo!("error for assigning to non-mut variable");
                        }
                        let addr = self
                            .locals
                            .get(&ident.def_id)
                            .and_then(|operand| operand.as_temp())
                            .copied()
                            .expect("addr for lhs");
                        let (val, block) = self.lower_expr(&assign.rhs, block)?;

                        self.get_block_mut(&block)
                            .push_instr(Instruction::Store { addr, val });
                    }
                    hir::Expr::Index(index) => {
                        let addr = self.new_temp(MirTy::new_ptr(index.ty));
                        let (base_val, block) = self.lower_expr(&index.expr, block)?;
                        let (index_val, block) = self.lower_expr(&index.idx, block)?;
                        let (val, block) = self.lower_expr(&assign.rhs, block)?;

                        let elem_ty = MirTy::new(index.expr.type_id().to_owned());

                        self.get_block_mut(&block)
                            .push_instr(Instruction::IndexPtr {
                                dest: addr,
                                base: base_val,
                                index: index_val,
                                elem_ty,
                            })
                            .push_instr(Instruction::Store { addr, val });
                    }
                    hir::Expr::MemberAccess(member_access) => {
                        let addr = self.new_temp(MirTy::new_ptr(member_access.ty));
                        // let ptr_dest = self.new_temp(member_access.ty);

                        let _lhs_ty_id = member_access.lhs.type_id();

                        let MemberResolution::Field(field_id) = member_access.resolution else {
                            todo!();
                        };

                        let (lval, block) = self.lower_expr(&member_access.lhs, block)?;
                        let (val, block) = self.lower_expr(&assign.rhs, block)?;

                        self.get_block_mut(&block)
                            .push_instr(Instruction::FieldPtr {
                                dest: addr,
                                lval,
                                field_id,
                                base_ty: MirTy::new(*member_access.lhs.type_id()),
                                field_ty: MirTy::new(member_access.ty),
                            })
                            .push_instr(Instruction::Store { addr, val });
                    }

                    _ => todo!("nice error for invalid lvalue"),
                }

                Ok((Operand::unit(), block))
            }

            hir::Expr::Loop(inner) => {
                let dest = self.new_temp(MirTy::new(inner.ty));
                let loop_block = self.new_block();
                let join_block = self.new_block();

                let builder = self.get_block_mut(&block);
                if !builder.is_terminated() {
                    builder.terminate(Terminator::Goto(loop_block))?;
                }

                self.loop_stack.enter(loop_block, join_block, dest);
                let (_body_val, body_exit) = self.lower_block(&inner.body, loop_block)?;
                self.loop_stack.leave();

                self.get_block_mut(&body_exit)
                    .terminate(Terminator::Goto(loop_block))?;

                Ok((Operand::Temporary(dest), join_block))
            }

            hir::Expr::Constructor(constructor) => {
                let dest = mem::take(&mut self.sret_dest).unwrap_or_else(|| {
                    let ty = MirTy::new(constructor.ty);
                    let dest = self.new_temp(ty);

                    self.get_block_mut(&block).push_instr(Instruction::Alloc {
                        dest,
                        ty,
                        count: 1,
                    });

                    dest
                });

                for (idx, (_, expr)) in constructor.fields.iter().enumerate() {
                    let field_ty_id = *expr.type_id();
                    let ptr_dest = self.new_temp(MirTy::new_ptr(field_ty_id));
                    let (val, block) = self.lower_expr(expr, block)?;

                    self.get_block_mut(&block)
                        .push_instr(Instruction::FieldPtr {
                            dest: ptr_dest,
                            lval: Operand::Temporary(dest),
                            field_id: FieldId::new(idx as u32),
                            base_ty: MirTy::new(constructor.ty),
                            field_ty: MirTy::new(field_ty_id),
                        })
                        .push_instr(Instruction::Store {
                            addr: ptr_dest,
                            val,
                        });
                }

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::MemberAccess(member_access) => {
                let field_ty = MirTy::new(member_access.ty);
                let dest = self.new_temp(field_ty);
                let ptr_dest = self.new_temp(MirTy::new(member_access.ty));

                let lhs_ty_id = member_access.lhs.type_id();
                let _lhs_ty = self.env.types.get(lhs_ty_id);
                // let (field_id, _) = lhs_ty
                //     .as_struct()
                //     .expect("member access can only be called on structs (for now)")
                //     .get_field(&member_access.rhs)
                //     .expect("field to exist");

                let MemberResolution::Field(field_id) = member_access.resolution else {
                    todo!();
                };

                let (lval, block) = self.lower_expr(&member_access.lhs, block)?;

                self.get_block_mut(&block)
                    .push_instr(Instruction::FieldPtr {
                        dest: ptr_dest,
                        lval,
                        field_id,
                        base_ty: MirTy::new(*member_access.lhs.type_id()),
                        field_ty,
                    })
                    .push_instr(Instruction::Load {
                        dest,
                        addr: ptr_dest,
                    });

                Ok((Operand::Temporary(dest), block))
            }

            hir::Expr::Ref(_expr) => todo!(),

            hir::Expr::Break(_inner) => {
                let ctx = self
                    .loop_stack
                    .current()
                    .copied()
                    .expect("break outside of loop context");

                self.get_block_mut(&block)
                    .terminate(Terminator::Goto(ctx.join_block))?;

                Ok((Operand::unit(), block))
            }

            hir::Expr::Continue(_) => {
                let ctx = self
                    .loop_stack
                    .current()
                    .copied()
                    .expect("continue outside of loop context");

                self.get_block_mut(&block)
                    .terminate(Terminator::Goto(ctx.enter_block))?;

                Ok((Operand::unit(), block))
            }
        }
    }
}
