use oxc_ast::{
    ast::{Argument, Expression, MemberExpression},
    AstKind,
};
use oxc_diagnostics::{
    miette::{self, Diagnostic},
    thiserror::Error,
};
use oxc_macros::declare_oxc_lint;
use oxc_span::Span;

use crate::{context::LintContext, rule::Rule, AstNode};

#[derive(Debug, Error, Diagnostic)]
#[error(
    "eslint-plugin-unicorn(prefer-reflect-apply): Prefer Reflect.apply() over Function#apply()"
)]
#[diagnostic(severity(warning), help("Reflect.apply() is less verbose and easier to understand."))]
struct PreferReflectApplyDiagnostic(#[label] pub Span);

#[derive(Debug, Default, Clone)]
pub struct PreferReflectApply;

declare_oxc_lint!(
    /// ### What it does
    ///
    ///
    /// ### Why is this bad?
    ///
    /// Reflect.apply() is arguably less verbose and easier to understand.
    /// In addition, when you accept arbitrary methods,
    /// it's not safe to assume .apply() exists or is not overridden.
    ///
    /// ### Example
    /// ```javascript
    /// // Bad
    /// foo.apply(null, [42]);
    ///
    /// // Good
    /// Reflect.apply(foo, null);
    /// ```
    PreferReflectApply,
    style
);

fn is_apply_signature(argument_1: &Argument, argument_2: &Argument) -> bool {
    let Argument::Expression(arg_expr_1) = argument_1 else {
        return false;
    };

    let Argument::Expression(arg_expr_2) = argument_2 else {
        return false;
    };

    if (matches!(arg_expr_1, Expression::ThisExpression(_))
        || matches!(arg_expr_1, Expression::NullLiteral(_)))
    {
        if matches!(arg_expr_2, Expression::ArrayExpression(_)) {
            return true;
        }

        if let Expression::Identifier(iden) = arg_expr_2 {
            return iden.name == "arguments";
        }
    }

    false
}

fn is_static_property_name_equal(expr: &MemberExpression, value: &str) -> bool {
    expr.static_property_name().is_some_and(|name| name == value)
}

impl Rule for PreferReflectApply {
    fn run<'a>(&self, node: &AstNode<'a>, ctx: &LintContext<'a>) {
        let AstKind::CallExpression(call_expr) = node.kind() else {
            return;
        };

        let Expression::MemberExpression(member_expr) = &call_expr.callee else {
            return;
        };

        if call_expr.optional
            || matches!(member_expr.object(), Expression::ArrayExpression(_))
            || matches!(member_expr.object(), Expression::ObjectExpression(_))
            || member_expr.object().is_literal()
        {
            return;
        }

        if is_static_property_name_equal(member_expr, "apply")
            && call_expr.arguments.len() == 2
            && is_apply_signature(&call_expr.arguments[0], &call_expr.arguments[1])
        {
            ctx.diagnostic(PreferReflectApplyDiagnostic(call_expr.span));
            return;
        }

        if is_static_property_name_equal(member_expr, "call") {
            if let Expression::MemberExpression(member_expr_obj) = member_expr.object() {
                if is_static_property_name_equal(member_expr_obj, "apply") {
                    if let Expression::MemberExpression(member_expr_obj_obj) =
                        member_expr_obj.object()
                    {
                        if is_static_property_name_equal(member_expr_obj_obj, "prototype") {
                            if let Expression::Identifier(iden) = member_expr_obj_obj.object() {
                                if iden.name == "Function"
                                    && call_expr.arguments.len() == 3
                                    && is_apply_signature(
                                        &call_expr.arguments[1],
                                        &call_expr.arguments[2],
                                    )
                                {
                                    ctx.diagnostic(PreferReflectApplyDiagnostic(call_expr.span));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn test() {
    use crate::tester::Tester;

    let pass = vec![
        ("foo.apply();", None),
        ("foo.apply(null);", None),
        ("foo.apply(this);", None),
        ("foo.apply(null, 42);", None),
        ("foo.apply(this, 42);", None),
        ("foo.apply(bar, arguments);", None),
        ("[].apply(null, [42]);", None),
        ("foo.apply(bar);", None),
        ("foo.apply(bar, []);", None),
        ("foo.apply;", None),
        ("apply;", None),
        ("Reflect.apply(foo, null);", None),
        ("Reflect.apply(foo, null, [bar]);", None),
        ("const apply = \"apply\"; foo[apply](null, [42]);", None),
    ];

    let fail = vec![
        ("foo.apply(null, [42]);", None),
        ("foo.bar.apply(null, [42]);", None),
        ("Function.prototype.apply.call(foo, null, [42]);", None),
        ("Function.prototype.apply.call(foo.bar, null, [42]);", None),
        ("foo.apply(null, arguments);", None),
        ("Function.prototype.apply.call(foo, null, arguments);", None),
        ("foo.apply(this, [42]);", None),
        ("Function.prototype.apply.call(foo, this, [42]);", None),
        ("foo.apply(this, arguments);", None),
        ("Function.prototype.apply.call(foo, this, arguments);", None),
        ("foo[\"apply\"](null, [42]);", None),
    ];

    Tester::new(PreferReflectApply::NAME, pass, fail).test_and_snapshot();
}