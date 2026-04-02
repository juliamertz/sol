use std::fmt::{Display, Formatter, Result, Write};

use super::*;

fn join_fmt(items: impl IntoIterator<Item = impl ToString>, sep: &str) -> String {
    items
        .into_iter()
        .map(|item| item.to_string())
        .collect::<Vec<_>>()
        .join(sep)
}

impl Display for Ident<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let (sigil, name) = match self {
            Ident::Ty(ty) => (':', ty),
            Ident::Global(global) => ('$', global),
            Ident::Temp(temp) => ('%', temp),
            Ident::Block(block) => ('@', block),
        };
        f.write_char(sigil)?;
        f.write_str(name)
    }
}

impl Display for BaseTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char(match self {
            BaseTy::Word => 'w',
            BaseTy::Long => 'l',
            BaseTy::Single => 's',
            BaseTy::Double => 'd',
        })
    }
}

impl Display for ExtTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char(match self {
            ExtTy::Base(base_ty) => return base_ty.fmt(f),
            ExtTy::Byte => 'b',
            ExtTy::HalfWord => 'h',
        })
    }
}

impl Display for SubWordTy {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(match self {
            SubWordTy::SignedByte => "sb",
            SubWordTy::UnsignedByte => "ub",
            SubWordTy::SingedHalf => "sh",
            SubWordTy::UnsingedHalf => "uh",
        })
    }
}

impl Display for AbiTy<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            AbiTy::Base(base_ty) => base_ty.fmt(f),
            AbiTy::SubWord(sub_word_ty) => sub_word_ty.fmt(f),
            AbiTy::Ident(ident) => f.write_str(ident),
        }
    }
}

impl Display for SubTyKind<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            SubTyKind::Extended(ext_ty) => ext_ty.fmt(f),
            SubTyKind::Ident(ident) => f.write_str(ident),
        }
    }
}

impl Display for SubTy<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.kind.fmt(f)?;
        if let Some(alignment) = self.align {
            f.write_char(' ')?;
            f.write_str(&alignment.to_string())?;
        }
        Ok(())
    }
}

impl Display for Ty<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "type :{} = ", self.ident())?;
        if let Some(alignment) = self.align() {
            write!(f, "align {alignment}")?;
        }

        f.write_char('{')?;
        match self {
            Ty::Aggregate { sub_tys, .. } => {
                for sub_ty in sub_tys {
                    sub_ty.fmt(f)?;
                    f.write_char(',')?;
                }
            }
            Ty::Union { variants, .. } => {
                f.write_char('{')?;
                for sub_tys in variants {
                    for sub_ty in sub_tys {
                        sub_ty.fmt(f)?;
                        f.write_char(',')?;
                    }
                }
                f.write_char('}')?;
            }
            Ty::Opaque { .. } => unimplemented!(),
        }
        f.write_char('}')
    }
}

impl Display for Linkage {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(match self {
            Linkage::Export => "export",
            Linkage::Thread => "thread",
        })
    }
}

impl Display for RegularParam<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.fmt(f)?;
        f.write_char(' ')?;
        self.1.fmt(f)
    }
}

impl Display for Param<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Param::Regular(param) => param.fmt(f),
            Param::Env(_) => unimplemented!(),
            Param::VariadicMarker => f.write_str("..."),
        }
    }
}

impl Display for Sign {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Sign::Minus => f.write_char('-'),
            Sign::None => Ok(()),
        }
    }
}

impl Display for Precision {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_char(match self {
            Precision::Single => 's',
            Precision::Double => 'd',
        })?;
        f.write_char('_')
    }
}

impl Display for Const<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Const::Int(sign, val) => write!(f, "{sign}{val}"),
            Const::Float(precision, val) => write!(f, "{precision}{val}"),
            Const::Ident(ident) => ident.fmt(f),
        }
    }
}

impl Display for Operand<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Operand::Var(name) => name.fmt(f),
            Operand::Const(constant) => constant.fmt(f),
        }
    }
}

impl Display for Instruction<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let (instr, operands) = match &self.kind {
            InstructionKind::Basic(kind, operands) => (*kind, join_fmt(operands, ", ")),
            InstructionKind::Call(ident, params) => (
                Instruction::CALL,
                format!("{ident}({})", join_fmt(params, ", ")),
            ),
        };

        write!(
            f,
            "{name} ={ty} {instr} {operands}",
            name = self.ident,
            ty = self.return_ty,
        )
    }
}

impl Display for Jump<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Jump::Jmp(ident) => write!(f, "jmp {ident}"),
            Jump::Jnz(operand, lhs, rhs) => {
                write!(f, "jnz {operand}, {lhs}, {rhs}")
            }
            Jump::Ret(operand) => write!(f, "ret {operand}"),
            Jump::Hlt => write!(f, "hlt"),
        }
    }
}

impl Display for Block<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.ident.fmt(f)?;
        f.write_char('\n')?;
        for instruction in self.instructions.iter() {
            writeln!(f, "\t{instruction}")?;
        }
        writeln!(f, "\t{}", self.jump)?;
        Ok(())
    }
}

impl Display for DataItem<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            DataItem::Ident(_ident, _) => todo!(),
            DataItem::String(val) => write!(f, "\"{val}\""),
            DataItem::Const(val) => val.fmt(f),
        }
    }
}

impl Display for DataValue<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            DataValue::Data(items) => f.write_str(
                &items
                    .iter()
                    .map(|(ty, item)| format!("{ty} {item}"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            DataValue::Zeroed(_) => todo!(),
        }
    }
}

impl Display for Data<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(
            f,
            "data {ident} = {{{val}}}",
            ident = self.ident,
            val = self.value
        )
    }
}

impl Display for Function<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if let Some(linkage) = self.linkage.as_ref() {
            linkage.fmt(f)?;
            f.write_char(' ')?;
        }
        f.write_str("function ")?;
        if let Some(return_ty) = self.return_ty.as_ref() {
            return_ty.fmt(f)?;
            f.write_char(' ')?;
        }
        self.ident.fmt(f)?;
        f.write_char('(')?;
        let params = join_fmt(&self.params, ", ");
        f.write_str(&params)?;
        f.write_char(')')?;
        f.write_char(' ')?;
        f.write_char('{')?;
        f.write_char('\n')?;
        for block in self.blocks.iter() {
            block.fmt(f)?;
            f.write_char('\n')?;
        }
        f.write_char('}')?;
        Ok(())
    }
}

impl Display for Definition<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Definition::Ty(ty) => ty.fmt(f),
            Definition::Data(data) => data.fmt(f),
            Definition::Fn(function) => function.fmt(f),
        }
    }
}

impl Display for Module<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for definition in self.defs.iter() {
            definition.fmt(f)?;
            f.write_char('\n')?;
        }
        Ok(())
    }
}
