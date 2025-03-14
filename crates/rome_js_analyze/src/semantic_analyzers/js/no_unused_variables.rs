use crate::semantic_services::Semantic;
use rome_analyze::{context::RuleContext, declare_rule, Rule, RuleCategory, RuleDiagnostic};
use rome_console::markup;
use rome_js_semantic::{AllReferencesExtensions, SemanticScopeExtensions};
use rome_js_syntax::{
    JsClassExpression, JsConstructorParameterList, JsConstructorParameters, JsFunctionDeclaration,
    JsFunctionExpression, JsIdentifierBinding, JsParameterList, JsParameters, JsSyntaxKind,
    JsVariableDeclarator,
};
use rome_rowan::AstNode;

declare_rule! {
    /// Disallow unused variables.
    ///
    /// ## Examples
    ///
    /// ### Invalid
    ///
    /// ```js,expect_diagnostic
    /// const a = 4;
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// let a = 4;
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// function foo() {
    /// };
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// function foo(myVar) {
    ///     console.log('foo');
    /// }
    /// foo();
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// const foo = () => {
    /// };
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// function foo() {
    ///     foo();
    /// }
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// const foo = () => {
    ///     foo();
    ///     console.log(this);
    /// };
    /// ```
    ///
    /// # Valid
    ///
    /// ```js
    /// function foo(b) {
    ///     console.log(b)
    /// };
    /// foo();
    /// ```
    ///
    /// ```js
    /// function foo(_unused) {
    /// };
    /// foo();
    /// ```
    ///
    /// ```jsx
    /// import React from 'react';
    /// function foo() {
    ///     return <div />;
    /// };
    /// foo();
    /// ```
    ///
    /// ```ts
    /// function used_overloaded(): number;
    /// function used_overloaded(s: string): string;
    /// function used_overloaded(s?: string) {
    ///     return s;
    /// }
    /// used_overloaded();
    /// ```
    pub(crate) NoUnusedVariables {
        version: "0.9.0",
        name: "noUnusedVariables",
        recommended: true,
    }
}

// It is ok in some Typescripts constructs for a parameter to be unused.
fn is_typescript_unused_ok(binding: &JsIdentifierBinding) -> Option<()> {
    match binding.syntax().parent()?.kind() {
        JsSyntaxKind::JS_FORMAL_PARAMETER | JsSyntaxKind::JS_REST_PARAMETER => {
            let parameter = binding.syntax().parent()?;
            let parent = parameter.parent()?;
            match parent.kind() {
                JsSyntaxKind::JS_PARAMETER_LIST => {
                    let parameters = JsParameterList::cast(parent)?.parent::<JsParameters>()?;
                    match parameters.syntax().parent()?.kind() {
                        JsSyntaxKind::TS_METHOD_SIGNATURE_CLASS_MEMBER
                        | JsSyntaxKind::TS_CALL_SIGNATURE_TYPE_MEMBER
                        | JsSyntaxKind::TS_METHOD_SIGNATURE_TYPE_MEMBER
                        | JsSyntaxKind::TS_FUNCTION_TYPE
                        | JsSyntaxKind::TS_DECLARE_FUNCTION_DECLARATION => Some(()),
                        _ => None,
                    }
                }
                JsSyntaxKind::JS_CONSTRUCTOR_PARAMETER_LIST => {
                    let parameters = JsConstructorParameterList::cast(parent)?
                        .parent::<JsConstructorParameters>()?;
                    match parameters.syntax().parent()?.kind() {
                        JsSyntaxKind::TS_CONSTRUCT_SIGNATURE_TYPE_MEMBER
                        | JsSyntaxKind::TS_CONSTRUCTOR_SIGNATURE_CLASS_MEMBER => Some(()),
                        _ => None,
                    }
                }
                JsSyntaxKind::TS_SETTER_SIGNATURE_TYPE_MEMBER
                | JsSyntaxKind::TS_SETTER_SIGNATURE_CLASS_MEMBER => Some(()),
                _ => None,
            }
        }
        JsSyntaxKind::TS_INDEX_SIGNATURE_PARAMETER => Some(()),
        JsSyntaxKind::TS_DECLARE_FUNCTION_DECLARATION => Some(()),
        _ => None,
    }
}

impl Rule for NoUnusedVariables {
    const CATEGORY: RuleCategory = RuleCategory::Lint;

    type Query = Semantic<JsIdentifierBinding>;
    type State = ();
    type Signals = Option<Self::State>;

    fn run(ctx: &RuleContext<Self>) -> Option<Self::State> {
        let binding = ctx.query();
        let name = binding.name_token().ok()?;
        let name = name.token_text_trimmed();
        let name = name.text();

        // Old code import React but do not used directly
        // only indirectly after transpiling JSX.
        if name.starts_with('_') || name == "React" {
            return None;
        }

        // Ignore expressions
        if binding.parent::<JsFunctionExpression>().is_some()
            || binding.parent::<JsClassExpression>().is_some()
        {
            return None;
        }

        if is_typescript_unused_ok(binding).is_some() {
            return None;
        }

        let model = ctx.model();
        if model.is_exported(binding) {
            return None;
        }

        let all_references = binding.all_references(model);
        if all_references.count() == 0 {
            Some(())
        } else {
            // We need to check if all uses of this binding are somehow recursive

            let function_declaration_scope = binding
                .parent::<JsFunctionDeclaration>()
                .map(|declaration| declaration.scope(model));

            let declarator = binding.parent::<JsVariableDeclarator>();

            let mut references_outside = 0;
            for r in binding.all_references(model) {
                let reference_scope = r.scope();

                // If this binding is a function, and all its references are "inside" this
                // function, we can safely say that this function is not used
                if function_declaration_scope
                    .as_ref()
                    .map(|s| s.is_ancestor_of(&reference_scope))
                    .unwrap_or(false)
                {
                    continue;
                }

                // Another possibility is if all its references are "inside" the same declaration
                if let Some(declarator) = declarator.as_ref() {
                    let node = declarator.syntax();
                    if r.node().ancestors().any(|n| n == *node) {
                        continue;
                    }
                }

                references_outside += 1;
                break;
            }

            if references_outside == 0 {
                Some(())
            } else {
                None
            }
        }
    }

    fn diagnostic(ctx: &RuleContext<Self>, _: &Self::State) -> Option<RuleDiagnostic> {
        let binding = ctx.query();

        let symbol_type = match binding.syntax().parent().unwrap().kind() {
            JsSyntaxKind::JS_FORMAL_PARAMETER => "parameter",
            JsSyntaxKind::JS_FUNCTION_DECLARATION => "function",
            JsSyntaxKind::JS_CLASS_DECLARATION => "class",
            JsSyntaxKind::TS_INTERFACE_DECLARATION => "interface",
            JsSyntaxKind::TS_TYPE_ALIAS_DECLARATION => "type alias",
            _ => "variable",
        };

        let diag = RuleDiagnostic::new(
            binding.syntax().text_trimmed_range(),
            markup! {
                "This " {symbol_type} " is unused."
            },
        );

        let diag = diag.footer_note(
            markup! {"Unused variables usually are result of incomplete refactoring, typos and other source of bugs."},
        );

        Some(diag)
    }
}
