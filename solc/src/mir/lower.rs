use crate::hir;
use crate::mir::builder::{Builder, BuilderError};
use crate::mir::{Module, Operand, Procedure, Terminator};
use crate::type_checker::{DefId, TypeEnv, TypeId};

fn lower_func(
    body: &hir::Block<'_>,
    params: &[(hir::Ident<'_>, TypeId)],
    env: &TypeEnv,
) -> Result<Procedure, BuilderError> {
    let mut builder = Builder::new(env);
    let entry = builder.new_block();


    for (ident, _ty_id) in params {
        let operand = Operand::Temporary(builder.new_temp());
        builder.define_local(ident.def_id, operand);
    }

    let (val, exit_block) = builder.lower_hir_block(body, entry)?;

    builder
        .get_block_mut(&exit_block)
        .terminate(Terminator::Return(val))?;
    builder.build()
}

pub fn lower_module(module: &hir::Module<'_>, env: &TypeEnv) -> Result<Module, BuilderError> {
    let procs = module
        .items
        .iter()
        .filter_map(|item| match item {
            hir::Item::Fn(hir::Fn {
                kind: hir::FnKind::Local { params, body },
                ..
            }) => Some(lower_func(body, params, env)),

            _ => None,
        })
        .collect::<Result<Vec<_>, BuilderError>>()?;

    Ok(Module { procs })
}
