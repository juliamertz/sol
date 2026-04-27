use std::fmt::{self, Write as _};

use crate::{
    interner::Id,
    mir::{visit::Visitor, *},
    type_checker::TypeEnv,
};

pub struct MirPrinter<'env> {
    env: &'env TypeEnv,
    module: Module,
}

impl<'env> MirPrinter<'env> {
    pub fn new(module: Module, env: &'env TypeEnv) -> Self {
        Self { env, module }
    }
}

impl fmt::Display for MirPrinter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fmt = FmtMir {
            f,
            env: self.env,
            depth: 0,
        };
        fmt.visit_module(&self.module)
    }
}

struct FmtMir<'env, 'f, 'w> {
    f: &'f mut fmt::Formatter<'w>,
    env: &'env crate::type_checker::TypeEnv,
    depth: usize,
}

impl FmtMir<'_, '_, '_> {
    fn write_indent(&mut self) -> fmt::Result {
        self.f.write_str("  ".repeat(self.depth).as_str())
    }

    fn enter(&mut self) {
        self.depth += 1;
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }
}

impl super::visit::Visitor<fmt::Result> for FmtMir<'_, '_, '_> {
    fn visit_module(&mut self, module: &Module) -> fmt::Result {
        for (idx, def) in module.defs.iter().enumerate() {
            self.visit_definition(def)?;
            if idx != module.defs.len() - 1 {
                writeln!(self.f)?;
            }
        }

        Ok(())
    }

    fn visit_definition(&mut self, def: &Definition) -> fmt::Result {
        match def {
            Definition::Ty(ty_def) => self.visit_ty_def(ty_def),
            Definition::Data(data) => self.visit_data(data),
            Definition::Fn(func) => self.visit_func(func),
        }
    }

    fn visit_ty_def(&mut self, ty_def: &TyDef) -> fmt::Result {
        match ty_def {
            TyDef::Struct { name, fields } => {
                writeln!(self.f, "struct {name} =")?;
                self.enter();
                for (field_id, mir_ty) in fields {
                    let ty = self.env.types.get(&mir_ty.inner);
                    self.write_indent()?;
                    writeln!(self.f, "{} = {ty}", field_id.into_inner())?;
                }
                self.leave();
                Ok(())
            },
        }
    }

    fn visit_data(&mut self, data: &Data) -> fmt::Result {
        write!(self.f, "_data_{}: ", data.id.into_inner())?;
        match data.value {
            DataValue::Bytes(_) => write!(self.f, "<bytes>"),
            DataValue::String(ref inner) => self.f.write_str(inner),
        }?;
        writeln!(self.f)
    }

    fn visit_func(&mut self, func: &Fn) -> fmt::Result {
        writeln!(self.f, "func @{}", func.name)?;

        self.enter();
        for (idx, block) in func.blocks.iter().enumerate() {
            self.write_indent()?;
            writeln!(self.f, "bb{idx}:")?;
            self.visit_block(block)?;
            if idx != func.blocks.len() - 1 {
                writeln!(self.f)?;
            }
        }
        self.leave();
        Ok(())
    }

    fn visit_block(&mut self, block: &Block) -> fmt::Result {
        self.enter();
        for instr in &block.body {
            self.write_indent()?;
            self.visit_instruction(instr)?;
            writeln!(self.f)?;
        }
        self.write_indent()?;
        self.visit_terminator(&block.term)?;
        writeln!(self.f)?;
        self.leave();
        Ok(())
    }

    fn visit_instruction(&mut self, instruction: &Instruction) -> fmt::Result {
        match instruction {
            Instruction::Copy { dest, val } => {
                self.visit_temp_id(dest)?;
                self.f.write_str(" = ")?;
                self.visit_operand(val)
            }
            Instruction::BinOp { dest, op, lhs, rhs } => {
                let op = match op {
                    BinOpKind::Eq => "eq",
                    BinOpKind::Ne => "ne",
                    BinOpKind::Add => "add",
                    BinOpKind::Sub => "sub",
                    BinOpKind::Mul => "mul",
                    BinOpKind::Div => "div",
                    BinOpKind::Lt => "lt",
                    BinOpKind::Gt => "gt",
                    BinOpKind::And => "and",
                    BinOpKind::Or => "or",
                };
                self.visit_temp_id(dest)?;
                write!(self.f, " = {op} ")?;
                self.visit_operand(lhs)?;
                self.f.write_str(", ")?;
                self.visit_operand(rhs)
            }
            Instruction::UnaryOp { dest, op, rhs } => {
                let op = match op {
                    UnaryOpKind::Negate => "neg",
                    UnaryOpKind::Not => "not",
                };
                self.visit_temp_id(dest)?;
                write!(self.f, " = {op} ")?;
                self.visit_operand(rhs)
            }
            Instruction::Call {
                dest,
                def,
                operands,
            } => {
                if let Some(dest) = dest {
                    self.visit_temp_id(dest)?;
                    self.f.write_str(" = ")?;
                }
                let def_name = self
                    .env
                    .def_names
                    .get(def).unwrap();
                write!(self.f, "call {def_name}(")?;
                for (idx, op) in operands.iter().enumerate() {
                    if idx > 0 {
                        self.f.write_str(", ")?;
                    }
                    self.visit_operand(op)?;
                }
                self.f.write_char(')')
            }
            Instruction::Alloc {
                dest,
                ty: mir_ty,
                count,
            } => {
                self.visit_temp_id(dest)?;
                write!(self.f, " = alloc{count} ")?;
                if mir_ty.indirection.is_ptr() {
                    self.f.write_char('*')?;
                }
                let ty = self.env.types.get(&mir_ty.inner);
                write!(self.f, "{ty}")
            }
            Instruction::Store { addr, val } => {
                self.f.write_str("store ")?;
                self.visit_operand(val)?;
                self.f.write_str(" -> ")?;
                self.visit_temp_id(addr)
            }
            Instruction::Load { dest, addr } => {
                self.visit_temp_id(dest)?;
                self.f.write_str(" = ")?;
                self.visit_temp_id(addr)
            }
            Instruction::IndexPtr {
                dest, base, index, ..
            } => {
                self.visit_temp_id(dest)?;
                self.f.write_str(" = index ")?;
                self.visit_operand(base)?;
                self.f.write_str(" -> ")?;
                self.visit_operand(index)
            }
            Instruction::FieldPtr {
                dest,
                lval,
                field_id,
                ..
            } => {
                self.visit_temp_id(dest)?;
                self.f.write_str(" = getfieldptr ")?;
                self.visit_operand(lval)?;
                write!(self.f, " -> {}", field_id.into_inner())
            }
        }
    }

    fn visit_terminator(&mut self, terminator: &Terminator) -> fmt::Result {
        match terminator {
            Terminator::Goto(block_id) => {
                write!(self.f, "goto ->")?;
                self.visit_block_id(block_id)
            }
            Terminator::Return(val) => {
                write!(self.f, "return ")?;
                self.visit_operand(val)
            }
            Terminator::Branch {
                cond,
                consequence,
                alternative,
            } => {
                self.f.write_str("branch ")?;
                self.visit_operand(cond)?;
                self.f.write_str(" -> ")?;
                self.visit_block_id(consequence)?;
                self.f.write_str(", ")?;
                self.visit_block_id(alternative)
            }
        }
    }

    fn visit_operand(&mut self, operand: &Operand) -> fmt::Result {
        match operand {
            Operand::Temporary(temp_id) => self.visit_temp_id(temp_id),
            Operand::Data(data_id) => self.visit_data_id(data_id),
            Operand::Constant(constant) => self.visit_constant(constant),
        }
    }

    fn visit_constant(&mut self, constant: &Constant) -> fmt::Result {
        match constant {
            Constant::Int(v, _) => write!(self.f, "{v}"),
            Constant::Float(v, _) => write!(self.f, "{v}"),
            Constant::Bool(v) => write!(self.f, "{v}"),
            Constant::Unit => self.f.write_str("()"),
        }
    }

    fn visit_temp_id(&mut self, temp_id: &TempId) -> fmt::Result {
        write!(self.f, "_{}", temp_id.into_inner())
    }

    fn visit_data_id(&mut self, data_id: &DataId) -> fmt::Result {
        write!(self.f, "_data_{}", data_id.into_inner())
    }

    fn visit_block_id(&mut self, block_id: &BlockId) -> fmt::Result {
        write!(self.f, "bb{}", block_id.into_inner())
    }
}
