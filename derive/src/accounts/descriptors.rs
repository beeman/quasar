use {
    super::semantics::{FieldSemantics, FieldShape, PdaSource, SeedNode},
    quasar_schema::{known_address_for_type, IdlAccountItem, IdlPda, IdlSeed},
    quote::ToTokens,
};

pub(crate) fn describe_accounts(semantics: &[FieldSemantics]) -> Vec<IdlAccountItem> {
    semantics
        .iter()
        .map(|sem| IdlAccountItem {
            name: sem.core.ident.to_string(),
            writable: sem.is_writable(),
            signer: matches!(sem.core.shape, FieldShape::Signer) || sem.client_requires_signer(),
            pda: sem.pda.as_ref().map(describe_pda),
            address: known_address(&sem.core.shape).map(str::to_owned),
        })
        .collect()
}

fn describe_pda(pda: &super::semantics::PdaConstraint) -> IdlPda {
    let seeds = match &pda.source {
        PdaSource::Raw { seeds } => seeds,
        PdaSource::Typed { args, .. } => args,
    };

    IdlPda {
        seeds: seeds.iter().map(describe_seed).collect(),
    }
}

fn describe_seed(seed: &SeedNode) -> IdlSeed {
    match seed {
        SeedNode::Literal(bytes) => IdlSeed::Const {
            value: bytes.clone(),
        },
        SeedNode::AccountAddress { field } => IdlSeed::Account {
            path: field.to_string(),
        },
        SeedNode::FieldBytes { root, path, .. } => IdlSeed::Account {
            path: join_path(root, path),
        },
        SeedNode::InstructionArg { name, .. } => IdlSeed::Arg {
            path: name.to_string(),
        },
        SeedNode::FieldRootedExpr { expr, .. } | SeedNode::OpaqueExpr(expr) => IdlSeed::Arg {
            path: expr.to_token_stream().to_string(),
        },
    }
}

fn join_path(root: &syn::Ident, path: &[syn::Ident]) -> String {
    let mut joined = root.to_string();
    for segment in path {
        joined.push('.');
        joined.push_str(&segment.to_string());
    }
    joined
}

fn known_address(shape: &FieldShape) -> Option<&'static str> {
    match shape {
        FieldShape::Program { .. } => {
            let inner = shape.inner_base_name().map(|name| name.to_string());
            known_address_for_type("Program", inner.as_deref())
        }
        FieldShape::Sysvar { .. } => {
            let inner = shape.inner_base_name().map(|name| name.to_string());
            known_address_for_type("Sysvar", inner.as_deref())
        }
        _ => None,
    }
}
