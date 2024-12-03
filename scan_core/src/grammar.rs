//! The language used by PGs and CSs.
//!
//! The type [`Expression<V>`] encodes the used language,
//! where `V` is the type parameter of variables.
//! The language features base types and product types,
//! Boolean logic and basic arithmetic expressions.

use ordered_float::OrderedFloat;
use std::hash::Hash;
use thiserror::Error;

/// The error type for operations with [`Type`].
#[derive(Debug, Clone, Copy, Error)]
pub enum TypeError {
    /// Types that should be matching are not,
    /// or are not compatible with each other.
    #[error("type mismatch")]
    TypeMismatch,
    /// The tuple has no component for such index.
    #[error("the tuple does not have the component")]
    MissingComponent,
    /// The variable's type is unknown.
    #[error("the type of variable is unknown")]
    UnknownVar,
    /// The index is out of bounds.
    #[error("the index is out of bounds")]
    IndexOutOfBounds,
}

/// The types supported by the language internally used by PGs and CSs.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Type {
    /// Boolean type.
    Boolean,
    /// Integer numerical type.
    Integer,
    /// Floating-point numerical type.
    Float,
    /// Product of a list of types (including other products).
    Product(Vec<Type>),
    /// List type
    List(Box<Type>),
}

impl Type {
    /// The default value for a given type.
    /// Used to initialize variables.
    pub fn default_value(&self) -> Val {
        match self {
            Type::Boolean => Val::Boolean(false),
            Type::Integer => Val::Integer(0),
            Type::Float => Val::Float(OrderedFloat(0.0)),
            Type::Product(tuple) => {
                Val::Tuple(Vec::from_iter(tuple.iter().map(Self::default_value)))
            }
            Type::List(t) => Val::List((**t).clone(), Vec::new()),
        }
    }
}

/// Integer values.
pub type Integer = i32;

/// Floating-point values.
pub type Float = f64;

/// Possible values for each [`Type`].
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Val {
    /// Boolean values.
    Boolean(bool),
    /// Integer values.
    Integer(Integer),
    /// Floating-point values.
    Float(OrderedFloat<Float>),
    /// Values for product types, i.e., tuples of suitable values.
    Tuple(Vec<Val>),
    /// Values for list types
    List(Type, Vec<Val>),
}

impl Val {
    pub fn r#type(&self) -> Type {
        match self {
            Val::Boolean(_) => Type::Boolean,
            Val::Integer(_) => Type::Integer,
            Val::Tuple(comps) => Type::Product(comps.iter().map(Val::r#type).collect()),
            Val::List(t, _) => Type::List(Box::new(t.clone())),
            Val::Float(_) => Type::Float,
        }
    }
}

impl From<Float> for Val {
    fn from(value: Float) -> Self {
        Val::Float(OrderedFloat(value))
    }
}

/// Expressions for the language internally used by PGs and CSs.
///
/// [`Expression<V>`] encodes the language in which `V` is the type of variables.
///
/// Note that not all expressions that can be formed are well-typed.
#[derive(Debug, Clone)]
pub enum Expression<V>
where
    V: Clone,
{
    // -------------------
    // General expressions
    // -------------------
    /// A constant value.
    Const(Val),
    /// A typed variable.
    Var(V, Type),
    /// A tuple of expressions.
    Tuple(Vec<Expression<V>>),
    /// The component of a tuple.
    Component(usize, Box<Expression<V>>),
    // -----------------
    // Logical operators
    // -----------------
    /// n-uary logical conjunction.
    And(Vec<Expression<V>>),
    /// n-uary logical disjunction.
    Or(Vec<Expression<V>>),
    /// Logical implication.
    Implies(Box<(Expression<V>, Expression<V>)>),
    /// Logical negation.
    Not(Box<Expression<V>>),
    // --------------------
    // Arithmetic operators
    // --------------------
    /// Opposite of a numerical expression.
    Opposite(Box<Expression<V>>),
    /// Arithmetic n-ary sum.
    Sum(Vec<Expression<V>>),
    /// Arithmetic n-ary multiplication.
    Mult(Vec<Expression<V>>),
    /// Mod operation
    Mod(Box<(Expression<V>, Expression<V>)>),
    // ------------
    // (In)Equality
    // ------------
    /// Equality of numerical expressions.
    Equal(Box<(Expression<V>, Expression<V>)>),
    /// Disequality of numerical expressions: LHS greater than RHS.
    Greater(Box<(Expression<V>, Expression<V>)>),
    /// Disequality of numerical expressions: LHS greater than, or equal to,  RHS.
    GreaterEq(Box<(Expression<V>, Expression<V>)>),
    /// Disequality of numerical expressions: LHS less than RHS.
    Less(Box<(Expression<V>, Expression<V>)>),
    /// Disequality of numerical expressions: LHS less than, or equal to, RHS.
    LessEq(Box<(Expression<V>, Expression<V>)>),
    // -----
    // Lists
    // -----
    /// Append element to the end of a list.
    Append(Box<(Expression<V>, Expression<V>)>),
    /// Truncate last element from a list.
    Truncate(Box<Expression<V>>),
    /// Take length of a list.
    Len(Box<Expression<V>>),
    // /// The component of a tuple.
    // Entry(Box<(Expression<V>, Expression<V>)>),
}

impl<V> Expression<V>
where
    V: Clone,
{
    // Computes the type of an expression.
    // Fails if the expression is badly typed,
    // e.g., if variables in it have type incompatible with the expression.
    pub fn r#type(&self) -> Result<Type, TypeError> {
        match self {
            Expression::Const(val) => Ok(val.r#type()),
            Expression::Tuple(tuple) => tuple
                .iter()
                .map(|e| e.r#type())
                .collect::<Result<Vec<Type>, TypeError>>()
                .map(Type::Product),
            Expression::Var(_var, t) => Ok(t.clone()),
            Expression::And(props) | Expression::Or(props) => {
                if props
                    .iter()
                    .map(|prop| prop.r#type())
                    .collect::<Result<Vec<Type>, TypeError>>()?
                    .iter()
                    .all(|prop| matches!(prop, Type::Boolean))
                {
                    Ok(Type::Boolean)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Implies(props) => {
                if matches!(props.0.r#type()?, Type::Boolean)
                    && matches!(props.1.r#type()?, Type::Boolean)
                {
                    Ok(Type::Boolean)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Not(prop) => {
                if matches!(prop.r#type()?, Type::Boolean) {
                    Ok(Type::Boolean)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Opposite(expr) => match expr.r#type()? {
                Type::Integer => Ok(Type::Integer),
                Type::Float => Ok(Type::Float),
                _ => Err(TypeError::TypeMismatch),
            },
            Expression::Sum(exprs) | Expression::Mult(exprs) => {
                let types = exprs
                    .iter()
                    .map(|expr| expr.r#type())
                    .collect::<Result<Vec<Type>, TypeError>>()?;

                if types.iter().all(|expr| matches!(expr, Type::Integer)) {
                    Ok(Type::Integer)
                } else if types
                    .iter()
                    .all(|expr| matches!(expr, Type::Integer | Type::Float))
                {
                    Ok(Type::Float)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Equal(exprs) | Expression::GreaterEq(exprs) | Expression::LessEq(exprs) => {
                let type_0 = exprs.0.r#type()?;
                let type_1 = exprs.1.r#type()?;
                if matches!(type_0, Type::Integer | Type::Boolean) && type_0 == type_1 {
                    Ok(Type::Boolean)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Greater(exprs) | Expression::Less(exprs) => {
                if matches!(exprs.0.r#type()?, Type::Integer | Type::Float)
                    && matches!(exprs.1.r#type()?, Type::Integer | Type::Float)
                {
                    Ok(Type::Boolean)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Component(index, expr) => {
                if let Type::Product(components) = expr.r#type()? {
                    components
                        .get(*index)
                        .cloned()
                        .ok_or(TypeError::MissingComponent)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Append(exprs) => {
                let list_type = exprs.0.r#type()?;
                let element_type = exprs.1.r#type()?;
                if let Type::List(ref elements_type) = list_type {
                    if &element_type == elements_type.as_ref() {
                        Ok(list_type)
                    } else {
                        Err(TypeError::TypeMismatch)
                    }
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Truncate(list) => {
                let list_type = list.r#type()?;
                if let Type::List(_) = list_type {
                    Ok(list_type)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Len(list) => {
                let list_type = list.r#type()?;
                if let Type::List(_) = list_type {
                    Ok(Type::Integer)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
            Expression::Mod(exprs) => {
                if matches!(exprs.0.r#type()?, Type::Integer)
                    && matches!(exprs.1.r#type()?, Type::Integer)
                {
                    Ok(Type::Integer)
                } else {
                    Err(TypeError::TypeMismatch)
                }
            }
        }
    }

    pub fn context(&self, vars: &dyn Fn(V) -> Option<Type>) -> Result<(), TypeError> {
        match self {
            Expression::Var(var, t) => {
                if let Some(var_t) = vars(var.clone()) {
                    if &var_t == t {
                        Ok(())
                    } else {
                        Err(TypeError::TypeMismatch)
                    }
                } else {
                    Err(TypeError::UnknownVar)
                }
            }
            Expression::Const(_) => Ok(()),
            Expression::Tuple(tuple)
            | Expression::And(tuple)
            | Expression::Or(tuple)
            | Expression::Sum(tuple)
            | Expression::Mult(tuple) => tuple.iter().try_for_each(|expr| expr.context(vars)),
            Expression::Component(_, expr)
            | Expression::Not(expr)
            | Expression::Opposite(expr)
            | Expression::Truncate(expr)
            | Expression::Len(expr) => expr.context(vars),
            Expression::Implies(exprs)
            | Expression::Equal(exprs)
            | Expression::Greater(exprs)
            | Expression::GreaterEq(exprs)
            | Expression::Less(exprs)
            | Expression::LessEq(exprs)
            | Expression::Mod(exprs)
            | Expression::Append(exprs) => {
                exprs.0.context(vars).and_then(|_| exprs.1.context(vars))
            }
        }
    }

    pub fn and(args: Vec<Self>) -> Self {
        match args.len() {
            0 => Expression::Const(Val::Boolean(true)),
            1 => args[0].clone(),
            _ => {
                let mut subformulae = Vec::new();
                for subformula in args.into_iter() {
                    if let Expression::And(subs) = subformula {
                        subformulae.extend(subs);
                    } else {
                        subformulae.push(subformula);
                    }
                }
                Expression::And(subformulae)
            }
        }
    }

    pub fn or(args: Vec<Self>) -> Self {
        match args.len() {
            0 => Expression::Const(Val::Boolean(false)),
            1 => args[0].clone(),
            _ => {
                let mut subformulae = Vec::new();
                for subformula in args.into_iter() {
                    if let Expression::Or(subs) = subformula {
                        subformulae.extend(subs);
                    } else {
                        subformulae.push(subformula);
                    }
                }
                Expression::Or(subformulae)
            }
        }
    }

    pub fn component(self, index: usize) -> Self {
        if let Expression::Tuple(args) = self {
            args[index].clone()
        } else {
            Expression::Component(index, Box::new(self))
        }
    }
}

impl<V> std::ops::Not for Expression<V>
where
    V: Clone,
{
    type Output = Self;

    fn not(self) -> Self::Output {
        if let Expression::Not(sub) = self {
            *sub
        } else {
            Expression::Not(Box::new(self))
        }
    }
}

impl<V> std::ops::Neg for Expression<V>
where
    V: Clone,
{
    type Output = Self;

    fn neg(self) -> Self::Output {
        if let Expression::Opposite(sub) = self {
            *sub
        } else {
            Expression::Opposite(Box::new(self))
        }
    }
}

impl<V> std::ops::Add for Expression<V>
where
    V: Clone,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut subformulae = Vec::new();
        if let Expression::Sum(subs) = self {
            subformulae.extend(subs);
        } else {
            subformulae.push(self);
        }
        if let Expression::Sum(subs) = rhs {
            subformulae.extend(subs);
        } else {
            subformulae.push(rhs);
        }
        Expression::Sum(subformulae)
    }
}

impl<V> std::ops::Mul for Expression<V>
where
    V: Clone,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut subformulae = Vec::new();
        if let Expression::Mult(subs) = self {
            subformulae.extend(subs);
        } else {
            subformulae.push(self);
        }
        if let Expression::Mult(subs) = rhs {
            subformulae.extend(subs);
        } else {
            subformulae.push(rhs);
        }
        Expression::Mult(subformulae)
    }
}

impl<V> std::iter::Sum for Expression<V>
where
    V: Clone,
{
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|acc, e| acc + e).unwrap_or(Self::from(0))
    }
}

impl<V> std::iter::Product for Expression<V>
where
    V: Clone,
{
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.reduce(|acc, e| acc * e).unwrap_or(Self::from(1))
    }
}

impl<V> From<bool> for Expression<V>
where
    V: Clone,
{
    fn from(value: bool) -> Self {
        Expression::Const(Val::Boolean(value))
    }
}

impl<V> From<Integer> for Expression<V>
where
    V: Clone,
{
    fn from(value: Integer) -> Self {
        Expression::Const(Val::Integer(value))
    }
}

impl<V> From<Float> for Expression<V>
where
    V: Clone,
{
    fn from(value: Float) -> Self {
        Expression::Const(Val::Float(OrderedFloat(value)))
    }
}

type DynFnExpr<V> = dyn Fn(&dyn Fn(V) -> Val) -> Val + Send + Sync;

pub(crate) struct FnExpression<V>(Box<DynFnExpr<V>>);

impl<C> std::fmt::Debug for FnExpression<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expression")
    }
}

impl<V> FnExpression<V> {
    #[inline(always)]
    pub fn eval(&self, vars: &dyn Fn(V) -> Val) -> Val {
        self.0(vars)
    }
}

impl<V: Clone + Copy + Send + Sync + 'static> From<Expression<V>> for FnExpression<V> {
    fn from(value: Expression<V>) -> Self {
        FnExpression(match value {
            Expression::Const(val) => Box::new(move |_| val.clone()),
            Expression::Var(var, _t) => Box::new(move |vars| {
                vars(var)
                // let val = vars(var);
                // if t == val.r#type() {
                //     val
                // } else {
                //     panic!("value and variable type mismatch");
                // }
            }),
            Expression::Tuple(exprs) => {
                let exprs: Vec<FnExpression<_>> =
                    exprs.into_iter().map(FnExpression::from).collect();
                Box::new(move |vars| {
                    Val::Tuple(exprs.iter().map(|expr| expr.eval(vars)).collect::<Vec<_>>())
                })
            }
            Expression::Component(index, expr) => {
                let expr = Self::from(*expr);
                Box::new(move |vars| {
                    if let Val::Tuple(vals) = expr.eval(vars) {
                        vals[index].clone()
                    } else {
                        panic!("index out of bounds");
                    }
                })
            }
            Expression::And(exprs) => {
                let exprs: Vec<FnExpression<_>> = exprs.into_iter().map(Self::from).collect();
                Box::new(move |vars| {
                    for expr in exprs.iter() {
                        if let Val::Boolean(b) = expr.eval(vars) {
                            if b {
                                continue;
                            } else {
                                return Val::Boolean(false);
                            }
                        } else {
                            panic!("type mismatch");
                        }
                    }
                    Val::Boolean(true)
                })
            }
            Expression::Or(exprs) => {
                let exprs: Vec<FnExpression<_>> = exprs.into_iter().map(Self::from).collect();
                Box::new(move |vars| {
                    for expr in exprs.iter() {
                        if let Val::Boolean(b) = expr.eval(vars) {
                            if b {
                                return Val::Boolean(true);
                            } else {
                                continue;
                            }
                        } else {
                            panic!("type mismatch");
                        }
                    }
                    Val::Boolean(false)
                })
            }
            Expression::Implies(exprs) => {
                let (lhs, rhs) = *exprs;
                let lhs = FnExpression::from(lhs);
                let rhs = FnExpression::from(rhs);
                Box::new(move |vars| {
                    if let (Val::Boolean(lhs), Val::Boolean(rhs)) = (lhs.eval(vars), rhs.eval(vars))
                    {
                        Val::Boolean(rhs || !lhs)
                    } else {
                        panic!("type mismatch");
                    }
                })
            }
            Expression::Not(expr) => {
                let expr = FnExpression::from(*expr);
                Box::new(move |vars| {
                    if let Val::Boolean(b) = expr.eval(vars) {
                        Val::Boolean(!b)
                    } else {
                        panic!("type mismatch");
                    }
                })
            }
            Expression::Opposite(expr) => {
                let expr = FnExpression::from(*expr);
                Box::new(move |vars| match expr.eval(vars) {
                    Val::Integer(i) => Val::Integer(-i),
                    Val::Float(f) => Val::Float(-f),
                    _ => panic!("type mismatch"),
                })
            }
            Expression::Sum(exprs) => {
                let exprs: Vec<FnExpression<_>> = exprs.into_iter().map(Self::from).collect();
                Box::new(move |vars| {
                    exprs.iter().fold(Val::Integer(0), |val, expr| match val {
                        Val::Integer(acc) => match expr.eval(vars) {
                            Val::Integer(i) => Val::Integer(acc + i),
                            Val::Float(f) => Val::Float(OrderedFloat::from(acc) + f),
                            _ => panic!("type mismatch"),
                        },
                        Val::Float(acc) => match expr.eval(vars) {
                            Val::Integer(i) => Val::Float(acc + OrderedFloat::from(i)),
                            Val::Float(f) => Val::Float(acc + f),
                            _ => panic!("type mismatch"),
                        },
                        _ => panic!("type mismatch"),
                    })
                })
            }
            Expression::Mult(exprs) => {
                let exprs: Vec<FnExpression<_>> = exprs.into_iter().map(Self::from).collect();
                Box::new(move |vars| {
                    exprs.iter().fold(Val::Integer(0), |val, expr| match val {
                        Val::Integer(acc) => match expr.eval(vars) {
                            Val::Integer(i) => Val::Integer(acc * i),
                            Val::Float(f) => Val::Float(OrderedFloat::from(acc) * f),
                            _ => panic!("type mismatch"),
                        },
                        Val::Float(acc) => match expr.eval(vars) {
                            Val::Integer(i) => Val::Float(acc * OrderedFloat::from(i)),
                            Val::Float(f) => Val::Float(acc * f),
                            _ => panic!("type mismatch"),
                        },
                        _ => panic!("type mismatch"),
                    })
                })
            }
            Expression::Equal(exprs) => {
                let (lhs, rhs) = *exprs;
                let lhs = FnExpression::from(lhs);
                let rhs = FnExpression::from(rhs);
                Box::new(move |vars| match (lhs.eval(vars), rhs.eval(vars)) {
                    (Val::Integer(lhs), Val::Integer(rhs)) => Val::Boolean(lhs == rhs),
                    (Val::Boolean(lhs), Val::Boolean(rhs)) => Val::Boolean(lhs == rhs),
                    _ => panic!("type mismatch"),
                })
            }
            Expression::Greater(exprs) => {
                let (lhs, rhs) = *exprs;
                let lhs = FnExpression::from(lhs);
                let rhs = FnExpression::from(rhs);
                Box::new(move |vars| match lhs.eval(vars) {
                    Val::Integer(lhs) => match rhs.eval(vars) {
                        Val::Integer(rhs) => Val::Boolean(lhs > rhs),
                        Val::Float(rhs) => Val::Boolean(OrderedFloat::from(lhs) > rhs),
                        _ => panic!("type mismatch"),
                    },
                    Val::Float(lhs) => match rhs.eval(vars) {
                        Val::Integer(rhs) => Val::Boolean(lhs > OrderedFloat::from(rhs)),
                        Val::Float(rhs) => Val::Boolean(lhs > rhs),
                        _ => panic!("type mismatch"),
                    },
                    _ => panic!("type mismatch"),
                })
            }
            Expression::GreaterEq(exprs) => {
                let (lhs, rhs) = *exprs;
                let lhs = FnExpression::from(lhs);
                let rhs = FnExpression::from(rhs);
                Box::new(move |vars| {
                    if let (Val::Integer(lhs), Val::Integer(rhs)) = (lhs.eval(vars), rhs.eval(vars))
                    {
                        Val::Boolean(lhs >= rhs)
                    } else {
                        panic!("type mismatch");
                    }
                })
            }
            Expression::Less(exprs) => {
                let (lhs, rhs) = *exprs;
                let lhs = FnExpression::from(lhs);
                let rhs = FnExpression::from(rhs);
                Box::new(move |vars| match lhs.eval(vars) {
                    Val::Integer(lhs) => match rhs.eval(vars) {
                        Val::Integer(rhs) => Val::Boolean(lhs < rhs),
                        Val::Float(rhs) => Val::Boolean(OrderedFloat::from(lhs) < rhs),
                        _ => panic!("type mismatch"),
                    },
                    Val::Float(lhs) => match rhs.eval(vars) {
                        Val::Integer(rhs) => Val::Boolean(lhs < OrderedFloat::from(rhs)),
                        Val::Float(rhs) => Val::Boolean(lhs < rhs),
                        _ => panic!("type mismatch"),
                    },
                    _ => panic!("type mismatch"),
                })
            }
            Expression::LessEq(exprs) => {
                let (source_lhs, source_rhs) = *exprs;
                let lhs = FnExpression::from(source_lhs);
                let rhs = FnExpression::from(source_rhs);
                Box::new(move |vars| {
                    if let (Val::Integer(lhs), Val::Integer(rhs)) = (lhs.eval(vars), rhs.eval(vars))
                    {
                        Val::Boolean(lhs <= rhs)
                    } else {
                        panic!("type mismatch");
                    }
                })
            }
            Expression::Append(exprs) => {
                let (list, element) = *exprs;
                let list = FnExpression::from(list);
                let element = FnExpression::from(element);
                Box::new(move |vars| {
                    if let Val::List(t, mut l) = list.eval(vars) {
                        let element = element.eval(vars);
                        if element.r#type() == t {
                            l.push(element);
                            Val::List(t, l)
                        } else {
                            panic!("type mismatch");
                        }
                    } else {
                        panic!("type mismatch");
                    }
                })
            }
            Expression::Truncate(list) => {
                let list = FnExpression::from(*list);
                Box::new(move |vars| {
                    if let Val::List(t, mut l) = list.eval(vars) {
                        if !l.is_empty() {
                            let _ = l.pop();
                            Val::List(t, l)
                        } else {
                            panic!("type mismatch");
                        }
                    } else {
                        panic!("type mismatch");
                    }
                })
            }
            Expression::Len(list) => {
                let list = FnExpression::from(*list);
                Box::new(move |vars| {
                    if let Val::List(_t, l) = list.eval(vars) {
                        Val::Integer(l.len() as Integer)
                    } else {
                        panic!("type mismatch");
                    }
                })
            }
            Expression::Mod(exprs) => {
                let (lhs, rhs) = *exprs;
                let lhs = FnExpression::from(lhs);
                let rhs = FnExpression::from(rhs);
                Box::new(move |vars| {
                    if let (Val::Integer(lhs), Val::Integer(rhs)) = (lhs.eval(vars), rhs.eval(vars))
                    {
                        Val::Integer(lhs % rhs)
                    } else {
                        panic!("type mismatch");
                    }
                })
            }
        })
    }
}
