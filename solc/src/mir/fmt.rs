use std::fmt::{self, Display, Write};

use crate::mir::*;

impl Display for TempId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_{}", self.0)
    }
}

impl Display for DataId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_data_{}", self.0)
    }
}

impl Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "bb{}", self.0)
    }
}

impl Display for Constant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Constant::Int(v, _) => write!(f, "{v}"),
            Constant::Bool(v) => write!(f, "{v}"),
            Constant::Unit => f.write_str("()"),
        }
    }
}

impl Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::Temporary(t) => write!(f, "{t}"),
            Operand::Data(d) => write!(f, "{d}"),
            Operand::Constant(c) => write!(f, "{c}"),
        }
    }
}

fn fmt_binop(op: &BinOpKind) -> &'static str {
    match op {
        BinOpKind::Eq => "eq",
        BinOpKind::Add => "add",
        BinOpKind::Sub => "sub",
        BinOpKind::Mul => "mul",
        BinOpKind::Div => "div",
        BinOpKind::Lt => "lt",
        BinOpKind::Gt => "gt",
        BinOpKind::And => "and",
        BinOpKind::Or => "or",
    }
}

fn fmt_unaryop(op: &UnaryOpKind) -> &'static str {
    match op {
        UnaryOpKind::Negate => "neg",
        UnaryOpKind::Not => "not",
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Copy { dest, val } => write!(f, "{dest} = {val}"),
            Instruction::BinOp { dest, op, lhs, rhs } => {
                write!(f, "{dest} = {} {lhs}, {rhs}", fmt_binop(op))
            }
            Instruction::UnaryOp { dest, op, rhs } => {
                write!(f, "{dest} = {} {rhs}", fmt_unaryop(op))
            }
            Instruction::Call {
                dest,
                def,
                operands,
            } => {
                write!(f, "{dest} = call def{}(", def.0)?;
                for (idx, op) in operands.iter().enumerate() {
                    if idx > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{op}")?;
                }
                f.write_char(')')
            }
        }
    }
}

impl Display for Terminator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Terminator::Goto(block) => write!(f, "goto -> {block}"),
            Terminator::Return(val) => write!(f, "return {val}"),
            Terminator::Branch {
                cond,
                consequence,
                alternative,
            } => {
                write!(f, "branch {cond} -> {consequence}, {alternative}")
            }
        }
    }
}

impl Display for Fn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "func @{}", self.name)?;

        for (idx, block) in self.blocks.iter().enumerate() {
            writeln!(f, "  bb{idx}:")?;
            for instr in &block.body {
                writeln!(f, "    {instr}")?;
            }
            writeln!(f, "    {}", block.term)?;
            if idx != self.blocks.len() - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

impl Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "_data_{}: ", self.id.inner())?;
        match self.value {
            DataValue::Bytes(_) => write!(f, "<bytes>"),
            DataValue::String(ref inner) => f.write_str(inner),
        }?;
        writeln!(f)
    }
}

impl Display for Definition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Definition::Ty(ty) => ty.fmt(f), 
            Definition::Data(data) => data.fmt(f),
            Definition::Fn(func) => func.fmt(f),
        }
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, def) in self.defs.iter().enumerate() {
            def.fmt(f)?;
            if idx != self.defs.len() - 1 {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
