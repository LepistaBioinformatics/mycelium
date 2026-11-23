use crate::{
    domain::{
        dtos::{error_code::ErrorCode, error_code_filters::ErrorCodeFilters},
        entities::ErrorCodeFetching,
    },
    use_cases::shared::error_codes::list_native_then_custom_error_codes,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// The largest page a public caller can request
pub const PUBLIC_ERROR_CODES_MAX_PAGE_SIZE: i32 = 100;

/// List the public (non-internal) error codes
///
/// No profile is required: only error codes with `is_internal = false` are
/// ever returned, and the page size is capped at
/// `PUBLIC_ERROR_CODES_MAX_PAGE_SIZE`. The in-code native error codes come
/// first, then the custom codes stored in the database.
#[tracing::instrument(name = "list_public_error_codes", skip_all)]
pub async fn list_public_error_codes(
    prefix: Option<String>,
    code: Option<i32>,
    page_size: Option<i32>,
    skip: Option<i32>,
    error_code_fetching_repo: Box<&dyn ErrorCodeFetching>,
) -> Result<FetchManyResponseKind<ErrorCode>, MappedErrors> {
    let capped_page_size =
        page_size.map(|size| size.clamp(1, PUBLIC_ERROR_CODES_MAX_PAGE_SIZE));

    let non_negative_skip = skip.map(|offset| offset.max(0));

    let filters = ErrorCodeFilters {
        prefix,
        code,
        is_internal: Some(false),
    };

    list_native_then_custom_error_codes(
        filters,
        capped_page_size,
        non_negative_skip,
        error_code_fetching_repo,
    )
    .await
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::MockErrorCodeFetching;

    /// A repository with no custom codes that checks every call lists public
    /// custom codes only
    fn public_customs_repo() -> MockErrorCodeFetching {
        let mut mock = MockErrorCodeFetching::new();

        mock.expect_list()
            .withf(|_, _, is_internal, is_native, _, _| {
                *is_internal == Some(false) && *is_native == Some(false)
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(FetchManyResponseKind::NotFound));

        mock
    }

    fn unpack(
        response: FetchManyResponseKind<ErrorCode>,
    ) -> (i64, Option<i64>, Option<i64>, Vec<ErrorCode>) {
        match response {
            FetchManyResponseKind::FoundPaginated {
                count,
                skip,
                size,
                records,
            } => (count, skip, size, records),
            other => panic!("expected a paginated response, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn never_returns_internal_natives() -> Result<(), MappedErrors> {
        let mock = public_customs_repo();

        let response = list_public_error_codes(
            None,
            None,
            Some(100),
            None,
            Box::new(&mock),
        )
        .await?;

        let (count, _, _, records) = unpack(response);

        assert_eq!(count, 30);
        assert_eq!(records.len(), 30);
        assert!(records.iter().all(|record| !record.is_internal));

        Ok(())
    }

    #[tokio::test]
    async fn caps_page_size_at_maximum() -> Result<(), MappedErrors> {
        let mock = public_customs_repo();

        let response = list_public_error_codes(
            None,
            None,
            Some(5000),
            None,
            Box::new(&mock),
        )
        .await?;

        assert_eq!(unpack(response).2, Some(100));

        Ok(())
    }

    #[tokio::test]
    async fn raises_non_positive_page_size_and_negative_skip(
    ) -> Result<(), MappedErrors> {
        let mock = public_customs_repo();

        let response = list_public_error_codes(
            None,
            None,
            Some(0),
            Some(-5),
            Box::new(&mock),
        )
        .await?;

        let (_, skip, size, records) = unpack(response);

        assert_eq!((skip, size), (Some(0), Some(1)));
        assert_eq!(records.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn uses_default_page_size_when_absent() -> Result<(), MappedErrors> {
        let mock = public_customs_repo();

        let response =
            list_public_error_codes(None, None, None, None, Box::new(&mock))
                .await?;

        assert_eq!(unpack(response).2, Some(10));

        Ok(())
    }
}
