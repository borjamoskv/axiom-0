//! Differential evidence for quantitative accounting. This deliberately slow,
//! recursive oracle carries an explicit usage vector for every subterm. It does
//! not use the elaborator's path scales or the production quantity operations.

use micro_axiom_0::{
    ast::{Ast, Expr, ExprId, Level, Quantity, Type, TypeId},
    elaborator::{self, Error},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Uses {
    None,
    Once,
    Many,
}

impl Uses {
    fn add(self, other: Self) -> Self {
        match (self, other) {
            (Self::None, x) | (x, Self::None) => x,
            _ => Self::Many,
        }
    }

    fn scale(self, quantity: Quantity) -> Self {
        match (quantity, self) {
            (Quantity::Zero, _) | (_, Self::None) => Self::None,
            (Quantity::One, x) => x,
            (Quantity::Omega, _) => Self::Many,
        }
    }

    fn allowed_by(self, quantity: Quantity) -> bool {
        match quantity {
            Quantity::Zero => self == Self::None,
            Quantity::One => self == Self::Once,
            Quantity::Omega => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Category {
    Unbound,
    CannotInfer,
    ExpectedFunction,
    Type,
    Quantity,
    Usage,
}

fn category(error: Error) -> Category {
    match error {
        Error::UnboundVariable { .. } => Category::Unbound,
        Error::CannotInferLambda(_) => Category::CannotInfer,
        Error::ExpectedFunction { .. } => Category::ExpectedFunction,
        Error::TypeMismatch { .. } => Category::Type,
        Error::QuantityMismatch { .. } => Category::Quantity,
        Error::UsageMismatch { .. } => Category::Usage,
        other => panic!("generator produced an invalid arena or shared occurrence: {other:?}"),
    }
}

type ReferenceResult = Result<(TypeId, Vec<Uses>), Category>;

fn reference_synthesize(ast: &Ast, root: ExprId, context: &[TypeId]) -> ReferenceResult {
    match ast.expr(root).unwrap() {
        Expr::Unit => Ok((ast.unit_type(), vec![Uses::None; context.len()])),
        Expr::Var(Level(level)) => {
            let ty = *context.get(level).ok_or(Category::Unbound)?;
            let mut usage = vec![Uses::None; context.len()];
            usage[level] = Uses::Once;
            Ok((ty, usage))
        }
        Expr::Lambda { .. } => Err(Category::CannotInfer),
        Expr::Ann { term, ty } => reference_check(ast, term, ty, context),
        Expr::App { function, argument } => {
            let (function_type, mut usage) = reference_synthesize(ast, function, context)?;
            let Type::Function {
                quantity,
                domain,
                codomain,
            } = ast.ty(function_type).unwrap()
            else {
                return Err(Category::ExpectedFunction);
            };
            // Check binders inside the argument locally before scaling its free
            // uses. In particular, erasure cannot excuse an invalid lambda.
            let (_, argument_usage) = reference_check(ast, argument, domain, context)?;
            assert_eq!(usage.len(), argument_usage.len());
            for (function_use, argument_use) in usage.iter_mut().zip(argument_usage) {
                *function_use = function_use.add(argument_use.scale(quantity));
            }
            Ok((codomain, usage))
        }
    }
}

fn reference_check(
    ast: &Ast,
    root: ExprId,
    expected: TypeId,
    context: &[TypeId],
) -> ReferenceResult {
    if let Expr::Lambda { quantity, body } = ast.expr(root).unwrap() {
        let Type::Function {
            quantity: declared,
            domain,
            codomain,
        } = ast.ty(expected).unwrap()
        else {
            return Err(Category::ExpectedFunction);
        };
        if quantity != declared {
            return Err(Category::Quantity);
        }
        let mut extended = context.to_vec();
        extended.push(domain);
        let (_, mut usage) = reference_check(ast, body, codomain, &extended)?;
        let local_usage = usage.pop().unwrap();
        if !local_usage.allowed_by(declared) {
            return Err(Category::Usage);
        }
        Ok((expected, usage))
    } else {
        let (found, usage) = reference_synthesize(ast, root, context)?;
        if found != expected {
            return Err(Category::Type);
        }
        Ok((found, usage))
    }
}

/// A fixed, dependency-free generator. The seed printed on failure completely
/// determines a case; this is not an entropy source or a statistical test.
struct Generator(u64);

impl Generator {
    fn pick(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound as u64) as usize
    }

    fn quantity(&mut self) -> Quantity {
        [Quantity::Zero, Quantity::One, Quantity::Omega][self.pick(3)]
    }
}

fn type_pool(ast: &mut Ast) -> Vec<TypeId> {
    let unit = ast.unit_type();
    let mut types = vec![unit];
    for quantity in [Quantity::Zero, Quantity::One, Quantity::Omega] {
        types.push(ast.function_type(quantity, unit, unit).unwrap());
    }
    let first_order = [types[1], types[2], types[3]];
    for quantity in [Quantity::Zero, Quantity::One, Quantity::Omega] {
        for domain in first_order {
            types.push(ast.function_type(quantity, domain, unit).unwrap());
            types.push(ast.function_type(quantity, unit, domain).unwrap());
        }
    }
    types
}

fn node(ast: &mut Ast, expr: Expr) -> ExprId {
    ast.push(expr).unwrap()
}

/// Generate type-shaped terms without enforcing resource usage. This reaches
/// successful typing and local usage checks much more often than syntax noise.
/// Every recursive call allocates new expression occurrences, even for variables.
fn shaped_term(
    ast: &mut Ast,
    generator: &mut Generator,
    types: &[TypeId],
    context: &mut Vec<TypeId>,
    expected: TypeId,
    fuel: usize,
) -> ExprId {
    let choice = generator.pick(6);
    if fuel > 0 && choice == 0 {
        let term = shaped_term(ast, generator, types, context, expected, fuel - 1);
        return node(ast, Expr::Ann { term, ty: expected });
    }
    if fuel > 0 && choice == 1 {
        let domain = types[generator.pick(4)];
        let function_type = ast
            .function_type(generator.quantity(), domain, expected)
            .unwrap();
        let function = shaped_term(ast, generator, types, context, function_type, fuel - 1);
        let function = node(
            ast,
            Expr::Ann {
                term: function,
                ty: function_type,
            },
        );
        let argument = shaped_term(ast, generator, types, context, domain, fuel - 1);
        return node(ast, Expr::App { function, argument });
    }
    let matching: Vec<_> = context
        .iter()
        .enumerate()
        .filter_map(|(level, ty)| (*ty == expected).then_some(level))
        .collect();
    if !matching.is_empty() && choice >= 3 {
        let level = matching[generator.pick(matching.len())];
        return node(ast, Expr::Var(Level(level)));
    }
    match ast.ty(expected).unwrap() {
        Type::Unit => node(ast, Expr::Unit),
        Type::Function {
            quantity,
            domain,
            codomain,
        } => {
            context.push(domain);
            let body = shaped_term(
                ast,
                generator,
                types,
                context,
                codomain,
                fuel.saturating_sub(1),
            );
            context.pop();
            node(ast, Expr::Lambda { quantity, body })
        }
    }
}

fn arbitrary_term(
    ast: &mut Ast,
    generator: &mut Generator,
    types: &[TypeId],
    depth: usize,
    scope: usize,
) -> ExprId {
    let choice = generator.pick(if depth == 0 { 2 } else { 5 });
    let expression = match choice {
        0 => Expr::Unit,
        1 => Expr::Var(Level(generator.pick(scope + 2))),
        2 => {
            let body = arbitrary_term(ast, generator, types, depth - 1, scope + 1);
            Expr::Lambda {
                quantity: generator.quantity(),
                body,
            }
        }
        3 => {
            let function = arbitrary_term(ast, generator, types, depth - 1, scope);
            let argument = arbitrary_term(ast, generator, types, depth - 1, scope);
            Expr::App { function, argument }
        }
        _ => {
            let term = arbitrary_term(ast, generator, types, depth - 1, scope);
            Expr::Ann {
                term,
                ty: types[generator.pick(types.len())],
            }
        }
    };
    node(ast, expression)
}

#[derive(Default)]
struct Coverage {
    accepted: usize,
    errors: [usize; 6],
}

impl Coverage {
    fn compare(
        &mut self,
        seed: u64,
        mode: &str,
        actual: Result<elaborator::Elaboration, Error>,
        reference: ReferenceResult,
    ) {
        if let Ok(result) = &actual {
            assert!(result.processed_frames() <= 5 * result.visited_nodes);
        }
        let reference = reference.map(|(ty, usage)| {
            assert!(usage.is_empty(), "closed root retained a free usage vector");
            ty
        });
        assert_eq!(
            actual.map(|result| result.ty).map_err(category),
            reference,
            "seed={seed}, mode={mode}"
        );
        match reference {
            Ok(_) => self.accepted += 1,
            Err(error) => self.errors[error as usize] += 1,
        }
    }
}

#[test]
fn explicit_usage_vectors_agree_on_six_thousand_type_shaped_terms() {
    let mut coverage = Coverage::default();
    for seed in 1..=6_000 {
        let mut ast = Ast::new();
        let types = type_pool(&mut ast);
        let mut generator = Generator(seed);
        let expected = types[generator.pick(types.len())];
        let root = shaped_term(
            &mut ast,
            &mut generator,
            &types,
            &mut Vec::new(),
            expected,
            4,
        );
        coverage.compare(
            seed,
            "check shaped",
            elaborator::check(&ast, root, expected),
            reference_check(&ast, root, expected, &[]),
        );
        let annotated = node(
            &mut ast,
            Expr::Ann {
                term: root,
                ty: expected,
            },
        );
        coverage.compare(
            seed,
            "synthesize annotation",
            elaborator::synthesize(&ast, annotated),
            reference_synthesize(&ast, annotated, &[]),
        );
        let alternative = types[generator.pick(types.len())];
        coverage.compare(
            seed,
            "check alternative",
            elaborator::check(&ast, annotated, alternative),
            reference_check(&ast, annotated, alternative, &[]),
        );
    }
    assert!(coverage.accepted > 1_000);
    assert!(coverage.errors[Category::Usage as usize] > 1_000);
    assert!(coverage.errors[Category::Type as usize] > 1_000);
}

#[test]
fn error_categories_agree_on_six_thousand_arbitrary_terms() {
    let mut coverage = Coverage::default();
    for seed in 1..=6_000 {
        let mut ast = Ast::new();
        let types = type_pool(&mut ast);
        let mut generator = Generator(seed ^ 0xa076_1d64_78bd_642f);
        let root = arbitrary_term(&mut ast, &mut generator, &types, 5, 0);
        let expected = types[generator.pick(types.len())];
        coverage.compare(
            seed,
            "synthesize arbitrary",
            elaborator::synthesize(&ast, root),
            reference_synthesize(&ast, root, &[]),
        );
        coverage.compare(
            seed,
            "check arbitrary",
            elaborator::check(&ast, root, expected),
            reference_check(&ast, root, expected, &[]),
        );
    }
    assert!(coverage.accepted > 500);
    for error in [
        Category::Unbound,
        Category::CannotInfer,
        Category::ExpectedFunction,
        Category::Type,
        Category::Quantity,
        Category::Usage,
    ] {
        assert!(
            coverage.errors[error as usize] > 0,
            "generator failed to exercise {error:?}"
        );
    }
}
