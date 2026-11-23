use super::list_matching_native_error_codes;
use crate::domain::{
    dtos::{error_code::ErrorCode, error_code_filters::ErrorCodeFilters},
    entities::ErrorCodeFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// The page size used when the caller does not request one
pub const DEFAULT_ERROR_CODES_PAGE_SIZE: i64 = 10;

/// List native error codes first, then custom ones, as one paginated result
///
/// `count` is the natives matching plus the customs matching. The natives
/// part of the page is served from memory and the remainder is read from the
/// database with the skip and size shifted past the natives. Native rows
/// stored in the database are excluded.
#[tracing::instrument(name = "list_native_then_custom_error_codes", skip_all)]
pub async fn list_native_then_custom_error_codes(
    filters: ErrorCodeFilters,
    page_size: Option<i32>,
    skip: Option<i32>,
    error_code_fetching_repo: Box<&dyn ErrorCodeFetching>,
) -> Result<FetchManyResponseKind<ErrorCode>, MappedErrors> {
    let (size, offset) = resolve_page(page_size, skip);
    let natives = list_matching_native_error_codes(&filters)?;
    let natives_count = natives.len() as i64;
    let mut records = page_of(natives, offset, size);

    let (customs_count, customs) = list_customs(
        filters,
        size - records.len() as i64,
        (offset - natives_count).max(0),
        error_code_fetching_repo,
    )
    .await?;

    records.extend(customs);

    Ok(FetchManyResponseKind::FoundPaginated {
        count: natives_count + customs_count,
        skip: Some(offset),
        size: Some(size),
        records,
    })
}

/// Resolve the requested page into a non-negative `(size, offset)`
fn resolve_page(page_size: Option<i32>, skip: Option<i32>) -> (i64, i64) {
    let size = page_size.map_or(DEFAULT_ERROR_CODES_PAGE_SIZE, i64::from);
    let offset = skip.map_or(0, i64::from);

    (size.max(0), offset.max(0))
}

fn page_of(natives: Vec<ErrorCode>, offset: i64, size: i64) -> Vec<ErrorCode> {
    natives
        .into_iter()
        .skip(offset as usize)
        .take(size as usize)
        .collect()
}

/// List the custom codes; a zero size still returns their total count
async fn list_customs(
    filters: ErrorCodeFilters,
    size: i64,
    skip: i64,
    error_code_fetching_repo: Box<&dyn ErrorCodeFetching>,
) -> Result<(i64, Vec<ErrorCode>), MappedErrors> {
    let response = error_code_fetching_repo
        .list(
            filters.prefix,
            filters.code,
            filters.is_internal,
            Some(false),
            Some(clamp_to_i32(size)),
            Some(clamp_to_i32(skip)),
        )
        .await?;

    Ok(count_and_records(response))
}

fn clamp_to_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

fn count_and_records(
    response: FetchManyResponseKind<ErrorCode>,
) -> (i64, Vec<ErrorCode>) {
    match response {
        FetchManyResponseKind::FoundPaginated { count, records, .. } => {
            (count, records)
        }
        FetchManyResponseKind::Found(records) => {
            (records.len() as i64, records)
        }
        FetchManyResponseKind::NotFound => (0, vec![]),
    }
}

// ? ---------------------------------------------------------------------------
// ? TESTS
// ? ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::native_error_codes::NativeErrorCodes,
        entities::MockErrorCodeFetching,
    };

    const CUSTOMS_COUNT: i64 = 25;

    fn natives_count() -> i64 {
        NativeErrorCodes::iter().count() as i64
    }

    fn custom(number: i32) -> ErrorCode {
        ErrorCode {
            prefix: "SVC".to_string(),
            error_number: number,
            code: None,
            message: "Custom failure".to_string(),
            details: None,
            is_internal: false,
            is_native: false,
        }
    }

    /// A repository holding `CUSTOMS_COUNT` customs that expects one call
    /// with the given size and skip, always excluding native rows
    fn customs_repo(
        expected_size: i32,
        expected_skip: i32,
    ) -> MockErrorCodeFetching {
        let mut mock = MockErrorCodeFetching::new();

        mock.expect_list()
            .withf(move |_, _, _, is_native, size, skip| {
                *is_native == Some(false)
                    && *size == Some(expected_size)
                    && *skip == Some(expected_skip)
            })
            .times(1)
            .returning(|_, _, _, _, size, skip| {
                let start = skip.unwrap_or(0);
                let end = (start + size.unwrap_or(0)).min(CUSTOMS_COUNT as i32);

                Ok(FetchManyResponseKind::FoundPaginated {
                    count: CUSTOMS_COUNT,
                    skip: skip.map(i64::from),
                    size: size.map(i64::from),
                    records: (start..end).map(custom).collect(),
                })
            });

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
    async fn serves_fully_native_page_and_still_counts_customs(
    ) -> Result<(), MappedErrors> {
        let mock = customs_repo(0, 0);

        let response = list_native_then_custom_error_codes(
            ErrorCodeFilters::default(),
            Some(5),
            Some(2),
            Box::new(&mock),
        )
        .await?;

        let (count, skip, size, records) = unpack(response);

        assert_eq!(count, natives_count() + CUSTOMS_COUNT);
        assert_eq!((skip, size), (Some(2), Some(5)));
        assert_eq!(records.len(), 5);
        assert!(records.iter().all(|record| record.is_native));
        assert_eq!(records[0].error_number, 3);

        Ok(())
    }

    #[tokio::test]
    async fn straddles_native_and_custom_boundary() -> Result<(), MappedErrors>
    {
        let skip = natives_count() - 3;
        let mock = customs_repo(7, 0);

        let response = list_native_then_custom_error_codes(
            ErrorCodeFilters::default(),
            Some(10),
            Some(skip as i32),
            Box::new(&mock),
        )
        .await?;

        let (count, _, _, records) = unpack(response);

        assert_eq!(count, natives_count() + CUSTOMS_COUNT);
        assert_eq!(records.len(), 10);
        assert!(records[..3].iter().all(|record| record.is_native));
        assert!(records[3..].iter().all(|record| !record.is_native));
        assert_eq!(records[3].error_number, 0);

        Ok(())
    }

    #[tokio::test]
    async fn serves_fully_custom_page_with_shifted_skip(
    ) -> Result<(), MappedErrors> {
        let skip = natives_count() + 4;
        let mock = customs_repo(10, 4);

        let response = list_native_then_custom_error_codes(
            ErrorCodeFilters::default(),
            Some(10),
            Some(skip as i32),
            Box::new(&mock),
        )
        .await?;

        let (count, _, _, records) = unpack(response);

        assert_eq!(count, natives_count() + CUSTOMS_COUNT);
        assert_eq!(records.len(), 10);
        assert_eq!(records[0].error_number, 4);

        Ok(())
    }

    #[tokio::test]
    async fn applies_filters_to_natives_and_defaults_page(
    ) -> Result<(), MappedErrors> {
        let mut mock = MockErrorCodeFetching::new();

        mock.expect_list()
            .withf(|prefix, code, is_internal, is_native, size, skip| {
                prefix.as_deref() == Some("myc")
                    && *code == Some(23)
                    && *is_internal == Some(false)
                    && *is_native == Some(false)
                    && *size == Some(9)
                    && *skip == Some(0)
            })
            .times(1)
            .returning(|_, _, _, _, _, _| Ok(FetchManyResponseKind::NotFound));

        let filters = ErrorCodeFilters {
            prefix: Some("myc".to_string()),
            code: Some(23),
            is_internal: Some(false),
        };

        let response = list_native_then_custom_error_codes(
            filters,
            None,
            Some(-3),
            Box::new(&mock),
        )
        .await?;

        let (count, skip, size, records) = unpack(response);

        assert_eq!(count, 1);
        assert_eq!((skip, size), (Some(0), Some(10)));
        assert_eq!(records[0].error_number, 23);

        Ok(())
    }
}
