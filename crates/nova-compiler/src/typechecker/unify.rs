use std::collections::HashMap;

use super::error::TypeError;
use super::types::{Type, TypeVarId};

/// Substitution map: type variable ID → resolved type
pub type Substitution = HashMap<TypeVarId, Type>;

/// Unify two types, returning a substitution that makes them equal.
///
/// This is the core of Hindley-Milner type inference. Given types A and B,
/// unification either finds a mapping of type variables that makes A = B,
/// or returns an error if the types are incompatible.
pub fn unify(a: &Type, b: &Type, subst: &mut Substitution) -> Result<(), TypeError> {
    let a = resolve(a, subst);
    let b = resolve(b, subst);

    match (&a, &b) {
        // Identical types unify trivially
        _ if a == b => Ok(()),

        // Type variable on either side — bind it
        (Type::Var(id), _) => bind(*id, &b, subst),
        (_, Type::Var(id)) => bind(*id, &a, subst),

        // Error type unifies with anything (error recovery)
        (Type::Error, _) | (_, Type::Error) => Ok(()),

        // Never type unifies with anything (it's the bottom type)
        (Type::Never, _) | (_, Type::Never) => Ok(()),

        // List[A] ~ List[B] → unify A with B
        (Type::List(a_inner), Type::List(b_inner)) => unify(a_inner, b_inner, subst),

        // Dict[K1,V1] ~ Dict[K2,V2] → unify K1~K2 and V1~V2
        (Type::Dict(ak, av), Type::Dict(bk, bv)) => {
            unify(ak, bk, subst)?;
            unify(av, bv, subst)
        }

        // Optional[A] ~ Optional[B]
        (Type::Optional(a_inner), Type::Optional(b_inner)) => unify(a_inner, b_inner, subst),

        // Result[A,E1] ~ Result[B,E2]
        (Type::Result(a_ok, a_err), Type::Result(b_ok, b_err)) => {
            unify(a_ok, b_ok, subst)?;
            unify(a_err, b_err, subst)
        }

        // Tuple — same length, element-wise unification
        (Type::Tuple(a_elems), Type::Tuple(b_elems)) if a_elems.len() == b_elems.len() => {
            for (ae, be) in a_elems.iter().zip(b_elems.iter()) {
                unify(ae, be, subst)?;
            }
            Ok(())
        }

        // Function types — same arity, unify params and return
        (
            Type::Function {
                params: a_params,
                return_type: a_ret,
                ..
            },
            Type::Function {
                params: b_params,
                return_type: b_ret,
                ..
            },
        ) if a_params.len() == b_params.len() => {
            for (ap, bp) in a_params.iter().zip(b_params.iter()) {
                unify(ap, bp, subst)?;
            }
            unify(a_ret, b_ret, subst)
        }

        // Struct types — same name
        (Type::Struct(a_struct), Type::Struct(b_struct)) if a_struct.name == b_struct.name => {
            Ok(())
        }

        // Enum types — same name
        (Type::Enum(a_enum), Type::Enum(b_enum)) if a_enum.name == b_enum.name => Ok(()),

        // Named types — same name (before resolution)
        (Type::Named(a_name), Type::Named(b_name)) if a_name == b_name => Ok(()),

        // Numeric coercion: int can widen to float
        (Type::Int, Type::Float) | (Type::Float, Type::Int) => Ok(()),

        // No match — type error
        _ => Err(TypeError::Mismatch {
            expected: a,
            found: b,
        }),
    }
}

/// Resolve a type through the substitution chain.
/// If the type is a variable that's been bound, follow the chain.
fn resolve(ty: &Type, subst: &Substitution) -> Type {
    match ty {
        Type::Var(id) => {
            if let Some(bound) = subst.get(id) {
                resolve(bound, subst)
            } else {
                ty.clone()
            }
        }
        _ => ty.clone(),
    }
}

/// Bind a type variable to a type, with occurs check.
fn bind(var_id: TypeVarId, ty: &Type, subst: &mut Substitution) -> Result<(), TypeError> {
    // Don't bind a variable to itself
    if let Type::Var(id) = ty {
        if *id == var_id {
            return Ok(());
        }
    }

    // Occurs check: prevent infinite types like T = List[T]
    if occurs(var_id, ty, subst) {
        return Err(TypeError::InfiniteType {
            var: format!("?T{var_id}"),
            ty: ty.clone(),
        });
    }

    subst.insert(var_id, ty.clone());
    Ok(())
}

/// Check if a type variable occurs within a type (prevents infinite types)
fn occurs(var_id: TypeVarId, ty: &Type, subst: &Substitution) -> bool {
    match ty {
        Type::Var(id) => {
            if *id == var_id {
                return true;
            }
            if let Some(bound) = subst.get(id) {
                occurs(var_id, bound, subst)
            } else {
                false
            }
        }
        Type::List(inner) | Type::Optional(inner) => occurs(var_id, inner, subst),
        Type::Dict(k, v) | Type::Result(k, v) => {
            occurs(var_id, k, subst) || occurs(var_id, v, subst)
        }
        Type::Tuple(elems) => elems.iter().any(|e| occurs(var_id, e, subst)),
        Type::Function {
            params,
            return_type,
            ..
        } => {
            params.iter().any(|p| occurs(var_id, p, subst))
                || occurs(var_id, return_type, subst)
        }
        _ => false,
    }
}

/// Apply the substitution to a type, resolving all variables to their final types.
/// Variables that remain unresolved default to their fallback type.
pub fn finalize(ty: &Type, subst: &Substitution) -> Type {
    let resolved = resolve(ty, subst);
    match resolved {
        Type::Var(_) => Type::Error, // Unresolved variable — could not infer
        Type::List(inner) => Type::List(Box::new(finalize(&inner, subst))),
        Type::Dict(k, v) => {
            Type::Dict(Box::new(finalize(&k, subst)), Box::new(finalize(&v, subst)))
        }
        Type::Optional(inner) => Type::Optional(Box::new(finalize(&inner, subst))),
        Type::Result(ok, err) => Type::Result(
            Box::new(finalize(&ok, subst)),
            Box::new(finalize(&err, subst)),
        ),
        Type::Tuple(elems) => Type::Tuple(elems.iter().map(|e| finalize(e, subst)).collect()),
        Type::Function {
            params,
            return_type,
            effects,
        } => Type::Function {
            params: params.iter().map(|p| finalize(p, subst)).collect(),
            return_type: Box::new(finalize(&return_type, subst)),
            effects,
        },
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unify_same_types() {
        let mut subst = Substitution::new();
        assert!(unify(&Type::Int, &Type::Int, &mut subst).is_ok());
        assert!(unify(&Type::Str, &Type::Str, &mut subst).is_ok());
    }

    #[test]
    fn test_unify_type_var() {
        let mut subst = Substitution::new();
        let var = Type::Var(99);
        assert!(unify(&var, &Type::Int, &mut subst).is_ok());
        assert_eq!(resolve(&var, &subst), Type::Int);
    }

    #[test]
    fn test_unify_mismatch() {
        let mut subst = Substitution::new();
        assert!(unify(&Type::Int, &Type::Str, &mut subst).is_err());
    }

    #[test]
    fn test_unify_list() {
        let mut subst = Substitution::new();
        let a = Type::List(Box::new(Type::Var(1)));
        let b = Type::List(Box::new(Type::Int));
        assert!(unify(&a, &b, &mut subst).is_ok());
        assert_eq!(resolve(&Type::Var(1), &subst), Type::Int);
    }

    #[test]
    fn test_unify_function() {
        let mut subst = Substitution::new();
        let a = Type::Function {
            params: vec![Type::Var(1)],
            return_type: Box::new(Type::Var(2)),
            effects: vec![],
        };
        let b = Type::Function {
            params: vec![Type::Int],
            return_type: Box::new(Type::Bool),
            effects: vec![],
        };
        assert!(unify(&a, &b, &mut subst).is_ok());
        assert_eq!(resolve(&Type::Var(1), &subst), Type::Int);
        assert_eq!(resolve(&Type::Var(2), &subst), Type::Bool);
    }

    #[test]
    fn test_occurs_check() {
        let mut subst = Substitution::new();
        let var = Type::Var(1);
        let recursive = Type::List(Box::new(Type::Var(1)));
        assert!(unify(&var, &recursive, &mut subst).is_err());
    }

    #[test]
    fn test_numeric_coercion() {
        let mut subst = Substitution::new();
        assert!(unify(&Type::Int, &Type::Float, &mut subst).is_ok());
    }
}
