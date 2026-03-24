use crate::ast;
use crate::hir::{self};
use crate::mir::{Block, BlockId, Instruction, Operand, TempId, Terminator};

#[derive(Debug, Default)]
struct BlockBuilder {
    body: Vec<Instruction>,
    term: Option<Terminator>,
}

impl BlockBuilder {
    fn push_instr(&mut self, instr: Instruction) -> &mut Self {
        self.body.push(instr);
        self
    }

    fn terminate(&mut self, term: Terminator) {
        if self.term.is_some() {
            panic!("may not terminate the same block more than once");
        }

        self.term = Some(term);
    }

    fn build(self) -> Block {
        Block {
            body: self.body,
            term: self.term.expect("unterminated block"),
        }
    }
}

pub struct Builder {
    temp_idx: usize,
    block_idx: usize,
    blocks: Vec<BlockBuilder>,
}

impl Builder {
    fn new_temp(&mut self) -> TempId {
        let id = TempId(self.temp_idx);
        self.temp_idx += 1;
        id
    }

    fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.block_idx);
        self.blocks.push(BlockBuilder::default());
        self.block_idx += 1;
        id
    }

    fn get_block_mut(&mut self, BlockId(idx): &BlockId) -> &mut BlockBuilder {
        &mut self.blocks[*idx]
    }

    fn lower_hir_block(
        &mut self,
        hir_block: &hir::Block<'_>,
        block: BlockId,
    ) -> (Operand, BlockId) {
        todo!()
    }

    fn lower_expr(&mut self, expr: &hir::Expr<'_>, block: BlockId) -> (Operand, BlockId) {
        match expr {
            hir::Expr::BinOp(hir::BinOp { op, lhs, rhs, .. }) => {
                let (lhs, block) = self.lower_expr(lhs, block);
                let (rhs, block) = self.lower_expr(rhs, block);
                let dest = self.new_temp();
                self.get_block_mut(&block)
                    .push_instr(Instruction::bin_op(dest, op.kind, lhs, rhs));

                (Operand::Temporary(dest), block)
            }

            hir::Expr::IfElse(if_else) => {
                let (cond, block) = self.lower_expr(&if_else.condition, block);
                let conseq_block = self.new_block();
                let alt_block = self.new_block();
                self.get_block_mut(&block).terminate(Terminator::branch(
                    cond,
                    conseq_block,
                    alt_block,
                ));

                let dest = self.new_temp();
                let join_block = self.new_block();

                let (conseq_val, conseq_exit) =
                    self.lower_hir_block(&if_else.consequence, conseq_block);
                self.get_block_mut(&conseq_exit)
                    .push_instr(Instruction::copy(dest, conseq_val))
                    .terminate(Terminator::goto(join_block));

                let (alt_val, alt_exit) =
                    self.lower_hir_block(if_else.alternative.as_ref().unwrap(), alt_block);
                self.get_block_mut(&alt_exit)
                    .push_instr(Instruction::copy(dest, alt_val))
                    .terminate(Terminator::goto(join_block));

                (Operand::Temporary(dest), join_block)
            }

            hir::Expr::Literal(literal) => {
                match literal.kind {
                    ast::LiteralKind::Str(str) => todo!(),
                    ast::LiteralKind::Int(int) => todo!(),
                }
            },

            hir::Expr::Ident(ident) => todo!(),
            hir::Expr::Block(block) => todo!(),
            hir::Expr::Prefix(prefix) => todo!(),
            hir::Expr::Call(call) => todo!(),
            hir::Expr::Index(index) => todo!(),
            hir::Expr::List(list) => todo!(),
            hir::Expr::Constructor(constructor) => todo!(),
            hir::Expr::MemberAccess(member_access) => todo!(),
            hir::Expr::Ref(expr) => todo!(),
        }
    }
}
