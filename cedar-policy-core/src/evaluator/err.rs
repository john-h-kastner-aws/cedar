/*
 * Copyright 2022-2023 Amazon.com, Inc. or its affiliates. All Rights Reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::ast::*;
use itertools::Itertools;
use miette::Diagnostic;
use nonempty::{nonempty, NonEmpty};
use smol_str::SmolStr;
use std::sync::Arc;
use thiserror::Error;

/// An error generated while evaluating an expression
#[derive(Debug, PartialEq, Eq, Clone, Error)]
#[error("{error_kind}")]
pub struct EvaluationError {
    /// The kind of error that occurred
    error_kind: EvaluationErrorKind,
    /// Optional advice on how to fix the error
    advice: Option<String>,
}

// custom impl of `Diagnostic`: non-trivial implementation of `help()`,
// everything else forwarded to `.error_kind`
impl Diagnostic for EvaluationError {
    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        match (self.error_kind.help(), self.advice.as_ref()) {
            (Some(help), None) => Some(help),
            (None, Some(advice)) => Some(Box::new(advice)),
            (Some(help), Some(advice)) => Some(Box::new(format!("{help}; {advice}"))),
            (None, None) => None,
        }
    }

    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.error_kind.code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        self.error_kind.severity()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.error_kind.url()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.error_kind.source_code()
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.error_kind.labels()
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        self.error_kind.diagnostic_source()
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        self.error_kind.related()
    }
}

impl EvaluationError {
    /// Extract the kind of issue detected during evaluation
    pub fn error_kind(&self) -> &EvaluationErrorKind {
        &self.error_kind
    }

    /// Set the advice field of an error
    pub fn set_advice(&mut self, advice: String) {
        self.advice = Some(advice);
    }

    /// Construct a [`EntityDoesNotExist`] error
    pub(crate) fn entity_does_not_exist(euid: Arc<EntityUID>) -> Self {
        Self {
            error_kind: EvaluationErrorKind::EntityDoesNotExist(euid),
            advice: None,
        }
    }

    /// Construct a [`EntityAttrDoesNotExist`] error
    pub(crate) fn entity_attr_does_not_exist(entity: Arc<EntityUID>, attr: SmolStr) -> Self {
        Self {
            error_kind: EvaluationErrorKind::EntityAttrDoesNotExist { entity, attr },
            advice: None,
        }
    }

    /// Construct a [`UnspecifiedEntityAccess`] error
    pub(crate) fn unspecified_entity_access(attr: SmolStr) -> Self {
        Self {
            error_kind: EvaluationErrorKind::UnspecifiedEntityAccess(attr),
            advice: None,
        }
    }

    /// Construct a [`RecordAttrDoesNotExist`] error
    pub(crate) fn record_attr_does_not_exist(attr: SmolStr, alternatives: Vec<SmolStr>) -> Self {
        Self {
            error_kind: EvaluationErrorKind::RecordAttrDoesNotExist(attr, alternatives),
            advice: None,
        }
    }

    /// Construct a [`TypeError`] error
    pub(crate) fn type_error(expected: NonEmpty<Type>, actual: Type) -> Self {
        Self {
            error_kind: EvaluationErrorKind::TypeError { expected, actual },
            advice: None,
        }
    }

    pub(crate) fn type_error_single(expected: Type, actual: Type) -> Self {
        Self::type_error(nonempty![expected], actual)
    }

    /// Construct a [`TypeError`] error with the advice field set
    pub(crate) fn type_error_with_advice(
        expected: NonEmpty<Type>,
        actual: Type,
        advice: String,
    ) -> Self {
        Self {
            error_kind: EvaluationErrorKind::TypeError { expected, actual },
            advice: Some(advice),
        }
    }

    pub(crate) fn type_error_with_advice_single(
        expected: Type,
        actual: Type,
        advice: String,
    ) -> Self {
        Self::type_error_with_advice(nonempty![expected], actual, advice)
    }

    /// Construct a [`WrongNumArguments`] error
    pub(crate) fn wrong_num_arguments(function_name: Name, expected: usize, actual: usize) -> Self {
        Self {
            error_kind: EvaluationErrorKind::WrongNumArguments {
                function_name,
                expected,
                actual,
            },
            advice: None,
        }
    }

    /// Construct a [`UnlinkedSlot`] error
    pub(crate) fn unlinked_slot(id: SlotId) -> Self {
        Self {
            error_kind: EvaluationErrorKind::UnlinkedSlot(id),
            advice: None,
        }
    }

    /// Construct a [`FailedExtensionFunctionApplication`] error
    pub(crate) fn failed_extension_function_application(extension_name: Name, msg: String) -> Self {
        Self {
            error_kind: EvaluationErrorKind::FailedExtensionFunctionApplication {
                extension_name,
                msg,
            },
            advice: None,
        }
    }

    /// Construct a [`NonValue`] error
    pub(crate) fn non_value(e: Expr) -> Self {
        Self {
            error_kind: EvaluationErrorKind::NonValue(e),
            advice: Some("consider using the partial evaluation APIs".into()),
        }
    }

    /// Construct a [`RecursionLimit`] error
    pub(crate) fn recursion_limit() -> Self {
        Self {
            error_kind: EvaluationErrorKind::RecursionLimit,
            advice: None,
        }
    }
}

impl From<crate::extensions::ExtensionFunctionLookupError> for EvaluationError {
    fn from(err: crate::extensions::ExtensionFunctionLookupError) -> Self {
        Self {
            error_kind: err.into(),
            advice: None,
        }
    }
}

impl From<IntegerOverflowError> for EvaluationError {
    fn from(err: IntegerOverflowError) -> Self {
        Self {
            error_kind: err.into(),
            advice: None,
        }
    }
}

impl From<RestrictedExprError> for EvaluationError {
    fn from(err: RestrictedExprError) -> Self {
        Self {
            error_kind: err.into(),
            advice: None,
        }
    }
}

/// Enumeration of the possible errors that can occur during evaluation
#[derive(Debug, PartialEq, Eq, Clone, Diagnostic, Error)]
pub enum EvaluationErrorKind {
    /// Tried to lookup this entity UID, but it didn't exist in the provided
    /// entities
    #[error("entity `{0}` does not exist")]
    EntityDoesNotExist(Arc<EntityUID>),

    /// Tried to get this attribute, but the specified entity didn't
    /// have that attribute
    #[error("`{}` does not have the attribute `{}`", &.entity, &.attr)]
    EntityAttrDoesNotExist {
        /// Entity that didn't have the attribute
        entity: Arc<EntityUID>,
        /// Name of the attribute it didn't have
        attr: SmolStr,
    },

    /// Tried to access an attribute of an unspecified entity
    #[error("cannot access attribute `{0}` of unspecified entity")]
    UnspecifiedEntityAccess(SmolStr),

    /// Tried to get an attribute of a (non-entity) record, but that record
    /// didn't have that attribute
    #[error("record does not have the attribute `{0}`")]
    #[diagnostic(help("available attributes: {1:?}"))]
    RecordAttrDoesNotExist(SmolStr, Vec<SmolStr>),

    /// An error occurred when looking up an extension function
    #[error(transparent)]
    #[diagnostic(transparent)]
    FailedExtensionFunctionLookup(#[from] crate::extensions::ExtensionFunctionLookupError),

    /// Tried to evaluate an operation on values with incorrect types for that
    /// operation
    #[error("{}", pretty_type_error(expected, actual))]
    TypeError {
        /// Expected one of these types
        expected: NonEmpty<Type>,
        /// Encountered this type instead
        actual: Type,
    },

    /// Wrong number of arguments provided to an extension function
    #[error("wrong number of arguments provided to extension function `{function_name}`: expected {expected}, got {actual}")]
    WrongNumArguments {
        /// arguments to this function
        function_name: Name,
        /// expected number of arguments
        expected: usize,
        /// actual number of arguments
        actual: usize,
    },

    /// Overflow during an integer operation
    #[error(transparent)]
    #[diagnostic(transparent)]
    IntegerOverflow(#[from] IntegerOverflowError),

    /// Error with the use of "restricted" expressions
    #[error(transparent)]
    #[diagnostic(transparent)]
    InvalidRestrictedExpression(#[from] RestrictedExprError),

    /// Thrown when a policy is evaluated with a slot that is not linked to an
    /// [`EntityUID`]
    #[error("template slot `{0}` was not linked")]
    UnlinkedSlot(SlotId),

    /// Evaluation error thrown by an extension function
    #[error("error while evaluating `{extension_name}` extension function: {msg}")]
    FailedExtensionFunctionApplication {
        /// Name of the extension throwing the error
        extension_name: Name,
        /// Error message from the extension
        msg: String,
    },

    /// This error is raised if an expression contains unknowns and cannot be
    /// reduced to a [`Value`]. In order to return partial results, use the
    /// partial evaluation APIs instead.
    #[error("the expression contains unknown(s): `{0}`")]
    NonValue(Expr),

    /// Maximum recursion limit reached for expression evaluation
    #[error("recursion limit reached")]
    RecursionLimit,
}

/// helper function for pretty-printing type errors
fn pretty_type_error(expected: &NonEmpty<Type>, actual: &Type) -> String {
    if expected.len() == 1 {
        format!("type error: expected {}, got {}", expected.first(), actual)
    } else {
        format!(
            "type error: expected one of [{}], got {actual}",
            expected.iter().join(", ")
        )
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Diagnostic, Error)]
pub enum IntegerOverflowError {
    /// Overflow during a binary operation
    #[error("integer overflow while attempting to {} the values `{arg1}` and `{arg2}`", match .op { BinaryOp::Add => "add", BinaryOp::Sub => "subtract", _ => "perform an operation on" })]
    BinaryOp {
        /// overflow while evaluating this operator
        op: BinaryOp,
        /// first argument to that operator
        arg1: Value,
        /// second argument to that operator
        arg2: Value,
    },

    /// Overflow during multiplication
    #[error("integer overflow while attempting to multiply `{arg}` by `{constant}`")]
    Multiplication {
        /// first argument, which wasn't necessarily a constant in the policy
        arg: Value,
        /// second argument, which was a constant in the policy
        constant: Integer,
    },

    /// Overflow during a unary operation
    #[error("integer overflow while attempting to {} the value `{arg}`", match .op { UnaryOp::Neg => "negate", _ => "perform an operation on" })]
    UnaryOp {
        /// overflow while evaluating this operator
        op: UnaryOp,
        /// argument to that operator
        arg: Value,
    },
}

/// Type alias for convenience
pub type Result<T> = std::result::Result<T, EvaluationError>;
