use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    Postgres,
    Sqlite,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterOp {
    #[default]
    Equal,
    NotEqual,
    Contains,
    Greater,
    Less,
    IsNull,
    IsNotNull,
}
impl FilterOp {
    pub const ALL: [Self; 7] = [
        Self::Equal,
        Self::NotEqual,
        Self::Contains,
        Self::Greater,
        Self::Less,
        Self::IsNull,
        Self::IsNotNull,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "≠",
            Self::Contains => "contient",
            Self::Greater => ">",
            Self::Less => "<",
            Self::IsNull => "est NULL",
            Self::IsNotNull => "non NULL",
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Filter {
    pub column: String,
    pub op: FilterOp,
    pub value: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sort {
    pub column: String,
    pub descending: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PageQuery {
    pub filters: Vec<Filter>,
    pub sort: Vec<Sort>,
    pub limit: usize,
    pub offset: usize,
    pub after: Option<Vec<Value>>,
}
impl Default for PageQuery {
    fn default() -> Self {
        Self {
            filters: vec![],
            sort: vec![],
            limit: 200,
            offset: 0,
            after: None,
        }
    }
}
pub struct BuiltQuery {
    pub sql: String,
    pub params: Vec<Value>,
}
pub fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
pub fn qualified(id: &ObjectId) -> String {
    format!("{}.{}", quote(&id.schema), quote(&id.name))
}
pub fn column<'a>(table: &'a Table, name: &str) -> Result<&'a Column> {
    table
        .columns
        .iter()
        .find(|c| c.name == name)
        .ok_or_else(|| Error::Invalid(format!("Colonne inconnue : {name}")))
}
pub fn effective_sort(table: &Table, query: &PageQuery) -> Vec<Sort> {
    let mut order = query.sort.clone();
    for name in table.identity_columns() {
        if !order.iter().any(|s| s.column == name) {
            order.push(Sort {
                column: name,
                descending: false,
            });
        }
    }
    order
}
pub fn cursor_for(table: &Table, query: &PageQuery, row: &DataRow) -> Option<Vec<Value>> {
    if table.identity_columns().is_empty() {
        return None;
    }
    let values: Vec<_> = effective_sort(table, query)
        .iter()
        .map(|s| table.column_index(&s.column).map(|i| row.values[i].clone()))
        .collect::<Option<_>>()?;
    if values.iter().any(|v| matches!(v, Value::Null)) {
        None
    } else {
        Some(values)
    }
}
pub fn build_page_query(table: &Table, query: &PageQuery, dialect: Dialect) -> Result<BuiltQuery> {
    let mut params = vec![];
    let mut clauses = vec![];
    let mut bind = |value: Value, col: &Column| {
        params.push(value);
        match dialect {
            Dialect::Postgres => format!("${}::text::{}", params.len(), col.data_type),
            Dialect::Sqlite => format!("?{}", params.len()),
        }
    };
    for filter in &query.filters {
        let c = column(table, &filter.column)?;
        let name = quote(&c.name);
        clauses.push(match filter.op {
            FilterOp::IsNull => format!("{name} IS NULL"),
            FilterOp::IsNotNull => format!("{name} IS NOT NULL"),
            FilterOp::Contains => {
                let text_col = Column::new("", "text", false);
                let placeholder = bind(
                    Value::Text(format!(
                        "%{}%",
                        filter
                            .value
                            .replace('\\', "\\\\")
                            .replace('%', "\\%")
                            .replace('_', "\\_")
                    )),
                    &text_col,
                );
                format!("CAST({name} AS TEXT) LIKE {placeholder} ESCAPE '\\'")
            }
            op => {
                let symbol = match op {
                    FilterOp::Equal => "=",
                    FilterOp::NotEqual => "<>",
                    FilterOp::Greater => ">",
                    _ => "<",
                };
                format!(
                    "{name} {symbol} {}",
                    bind(Value::Text(filter.value.clone()), c)
                )
            }
        });
    }
    let order = effective_sort(table, query);
    for sort in &order {
        column(table, &sort.column)?;
    }
    if let Some(after) = &query.after {
        if after.len() != order.len()
            || table.identity_columns().is_empty()
            || after.iter().any(|v| matches!(v, Value::Null))
        {
            return Err(Error::Invalid("Curseur de pagination invalide".into()));
        }
        let placeholders: Vec<_> = order
            .iter()
            .zip(after)
            .map(|(s, value)| {
                bind(
                    value.clone(),
                    column(table, &s.column).expect("validated column"),
                )
            })
            .collect();
        let mut disjunction = vec![];
        for (i, s) in order.iter().enumerate() {
            let mut terms: Vec<_> = (0..i)
                .map(|j| format!("{} = {}", quote(&order[j].column), placeholders[j]))
                .collect();
            let comparison = format!(
                "{} {} {}",
                quote(&s.column),
                if s.descending { "<" } else { ">" },
                placeholders[i],
            );
            terms.push(if column(table, &s.column)?.nullable {
                format!("({comparison} OR {} IS NULL)", quote(&s.column))
            } else {
                comparison
            });
            disjunction.push(format!("({})", terms.join(" AND ")));
        }
        clauses.push(format!("({})", disjunction.join(" OR ")));
    }
    let fields = table
        .columns
        .iter()
        .map(|c| match dialect {
            Dialect::Postgres => format!("{}::text", quote(&c.name)),
            Dialect::Sqlite => quote(&c.name),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let version = if dialect == Dialect::Postgres && !table.is_view {
        ", xmin::text"
    } else {
        ""
    };
    let mut sql = format!("SELECT {fields}{version} FROM {}", qualified(&table.id));
    if !clauses.is_empty() {
        sql.push_str(&format!(" WHERE {}", clauses.join(" AND ")));
    }
    if !order.is_empty() {
        sql.push_str(&format!(
            " ORDER BY {}",
            order
                .iter()
                .map(|s| format!(
                    "{} {} NULLS LAST",
                    quote(&s.column),
                    if s.descending { "DESC" } else { "ASC" }
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    sql.push_str(&format!(
        " LIMIT {} OFFSET {}",
        query.limit.clamp(1, 2000) + 1,
        if query.after.is_some() {
            0
        } else {
            query.offset
        }
    ));
    Ok(BuiltQuery { sql, params })
}
