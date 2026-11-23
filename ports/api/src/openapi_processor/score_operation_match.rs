use crate::dtos::ToolOperation;

/// Score how well a tool operation matches the query terms
///
/// Every query term is weighted against the service name, the summary, each
/// tag and the path. The score is the average weight times 100: 0 means no
/// match, higher is better. Without query terms the score is 0.
///
pub(crate) fn score_operation_match(
    tool_operation: &ToolOperation,
    query_terms: &[String],
) -> i32 {
    let subjects = collect_match_subjects(tool_operation);

    let weights = query_terms
        .iter()
        .flat_map(|term| {
            subjects
                .iter()
                .map(move |subject| get_match_weight(term, subject))
        })
        .collect::<Vec<i32>>();

    if weights.is_empty() {
        return 0;
    }

    (weights.iter().sum::<i32>() * 100) / weights.len() as i32
}

fn collect_match_subjects(tool_operation: &ToolOperation) -> Vec<&str> {
    let mut subjects = vec![tool_operation.service.name.as_str()];
    subjects.extend(tool_operation.operation.summary.as_deref());
    subjects.extend(tool_operation.operation.tags.iter().map(String::as_str));
    subjects.push(tool_operation.path.as_str());
    subjects
}

/// Get the match weight
///
/// If the match is exact return 2, if one contains the other return 1,
/// otherwise return 0.
///
fn get_match_weight(query: &str, subject: &str) -> i32 {
    let query = query.to_lowercase();
    let subject = subject.to_lowercase();

    if query == subject {
        return 2;
    }

    if query.contains(&subject) || subject.contains(&query) {
        return 1;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi_processor::synthetic_catalog_fixture::build_synthetic_catalog;

    #[test]
    fn get_match_weight_ranks_exact_partial_and_missing_matches() {
        assert_eq!(get_match_weight("Records", "records"), 2);
        assert_eq!(get_match_weight("record", "records"), 1);
        assert_eq!(get_match_weight("accounts", "records"), 0);
    }

    #[test]
    fn score_operation_match_averages_weights_over_every_subject() {
        let catalog = build_synthetic_catalog(2, 1);
        let operation = &catalog.operations[1];
        let terms = vec!["service-02".to_string()];

        // service name (2) + summary (0) + tag (0) + path (1) over 4 subjects
        assert_eq!(score_operation_match(operation, &terms), 75);
        assert_eq!(score_operation_match(&catalog.operations[0], &terms), 0);
    }

    #[test]
    fn score_operation_match_is_zero_without_query_terms() {
        let catalog = build_synthetic_catalog(1, 1);

        assert_eq!(score_operation_match(&catalog.operations[0], &[]), 0);
    }
}
