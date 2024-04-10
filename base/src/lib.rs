/// Defines default Data Transfer Auxiliary structs
///
/// This module contains all the auxiliary structs used to transfer data between
/// layers. These structs are used to transfer data between layers and are not
/// intended to be used as entities.
///
/// # Examples
///
/// If you want to create two related structs with database representations and
/// the relationship between them is established by a foreign key, you can use
/// Parent and Children.
///
/// ```
/// use mycelium_base::dtos::{Children, Parent};
///
/// struct Post {
///    id: i32,
///    title: String,
///    comments: Children<Comment, i32>,
/// };
///
/// struct Comment {
///     post: Parent<Post, i32>,
///     id: i32,
///     text: String,
/// };
///
/// let post_with_comments_as_ids = Post {
///     id: 1,
///     title: "Hello World".to_string(),
///     comments: Children::Ids(vec![1, 2, 3]),
/// };
///
/// let post_with_comments_as_records = Post {
///     id: 1,
///     title: "Hello World".to_string(),
///     comments: Children::Records(vec![
///         Comment {
///             post: Parent::Id(1),
///             id: 1,
///             text: "Hello World from comment 1".to_string(),
///         },
///         Comment {
///             post: Parent::Id(1),
///             id: 2,
///             text: "Hello World from comment 2".to_string(),
///         },
///         Comment {
///             post: Parent::Id(1),
///             id: 3,
///             text: "Hello World from comment 3".to_string(),
///         },
///     ]),
/// };
/// ```
///
pub mod dtos;

/// Defines default entities for clean architecture based projects
///
/// This module contains all the entities used in the project. Entities are
/// the core of the project and are used to represent the business logic.
/// Entities are not intended to be used as Data Transfer Objects.
pub mod entities;

/// Defines common utilities for error management
///
/// This module contains all the utilities used to manage errors in the project.
/// These utilities are used to manage errors in a clean way.
pub mod utils;
