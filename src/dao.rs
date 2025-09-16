use sqlx::{any::AnyRow, Row, AnyPool};

use crate::recipes::{Element, ElementHandle};

#[derive(Debug, )]
pub enum Errors {
    ExpectOneResult {
        table_name: String,
    },
    FetchedZeroRow(String),
    ElementNotFound(String),
    SqlxError(sqlx::Error)
}

impl From<sqlx::Error> for Errors {
    fn from(value: sqlx::Error) -> Self {
        Self::SqlxError(value)
    }
}

impl snafu::Error for Errors {}

impl std::fmt::Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Errors::ExpectOneResult { table_name } => {
                write!(f, "Problematic table: {table_name}")
            },
            Errors::ElementNotFound(ele_name) => {
                write!(f, "Element: {ele_name}")
            }
            Errors::SqlxError(e) => {
                write!(f, "SqlxError: {e}")
            },
            Errors::FetchedZeroRow(e) => {
                write!(f, "Fetch zero rows: {e}")
            }
        }
    }
}

pub struct DAO {
    database: AnyPool,
}

impl DAO {
    pub async fn new_str(url: &'static str) -> Self {
        let database = AnyPool::connect(url)
            .await
            .expect("Database {url} connection failed.");
        let _a = sqlx::raw_sql(
            "PRAGMA foreign_keys = ON"
        )
            .execute(&database)
            .await
            .expect("The sqlite3's PRAGMA opened failed.");

        #[cfg(debug_assertions)]
        eprintln!("{_a:?}");
        Self {
            database
        }
    }

    pub async fn list_mods(&self) -> Result<Vec<String>, Errors> {
        let res =
            sqlx::query(
                "SELECT belongs_to_mod FROM elements GROUP BY belongs_to_mod ORDER BY belongs_to_mod"
            )
            .fetch_all(&self.database)
            .await?;
        let mut v = vec![];
        for x in res {
            v.push(
                x.try_get::<String, _>("belongs_to_mod")?
            );
        }
        Ok(v)
    }

    pub async fn list_recipes(&self) -> Result<Vec<(ElementHandle, ElementHandle, ElementHandle)>, Errors> {
        let res =
            sqlx::query(
                "SELECT name,component_a,component_b FROM recipes"
                )
            .fetch_all(&self.database)
            .await?;

        let mut v = Vec::new();
        for row in res {
            let name = row.try_get::<String, _>("name")?;
            let component_a = row.try_get::<String, _>("component_a")?;
            let component_b = row.try_get::<String, _>("component_b")?;
            v.push(
                (
                    ElementHandle::from(name),
                    ElementHandle::from(component_a), 
                    ElementHandle::from(component_b),
            ));
        }
        Ok(v)
    }

    pub async fn list_elements_holding(&self) -> Result<Vec<(ElementHandle, f64)>, Errors> {
        let res =
            sqlx::query(
                "SELECT name,num FROM elements_holding"
            )
            .fetch_all(&self.database)
            .await?;

        let mut v = vec![];
        for x in res {
            let name = x.try_get::<String, _>("name")?;
            let element_holding = x.try_get::<f64, _>("num")?;
            v.push((ElementHandle::from(name), element_holding));
        }
        Ok(v)
    }

    pub async fn list_elements(&self) -> Result<Vec<Element>, Errors> {
        let res =
            sqlx::query(
                "SELECT name,belongs_to_mod,base_value FROM elements"
                )
            .fetch_all(&self.database)
            .await?;

        let mut v = Vec::new();
        for row in res {
            let name = row.try_get::<String, _>("name")?;
            let belongs_to_mod = row.try_get::<Option<String>, _>("belongs_to_mod")?;
            let base_value = row.try_get::<f64, _>("base_value")?;
            v.push(
                Element {
                    name,
                    belongs_to_mod,
                    base_value
                }
            )
        }
        Ok(v)
    }

    pub async fn does_element_exists(&self, ele: &ElementHandle) -> Result<bool, Errors> {
        let res =
            sqlx::query(
                "SELECT count(*) as count_ FROM elements WHERE name=$1"
            )
            .bind(ele.get_name())
            .fetch_one(&self.database)
            .await?;

        let count = res.try_get::<i64, _>("count_")?;
        if count == 0 {
            return Ok(false);
        } else if count == 1 {
            return Ok(true);
        } else {
            return Err(Errors::ExpectOneResult { table_name: "elements".to_string() });
        }
    }

    pub async fn get_element_base_value(&self, ele: &ElementHandle) -> Result<f64, Errors> {
        let res = 
            sqlx::query(
                "SELECT base_value FROM elements WHERE name=$1"
            )
            .bind(ele.get_name())
            .fetch_all(&self.database)
            .await?;

        if res.len() == 1 {
            let bv = res.get(0).unwrap().try_get::<f64, _>("base_value")
                .unwrap();
            Ok(bv)
        } else {
            Err(Errors::ExpectOneResult { table_name: format!("elements: name={}", ele.get_name()) })
        }
    }

    pub async fn get_element_num_holding(&self, handle: &ElementHandle) -> Result<f64, Errors> {
        let res = sqlx::query(
            "SELECT num FROM elements_holding WHERE name=$1"
        )
            .bind(handle.get_name())
            .fetch_all(&self.database)
            .await?;
        if res.len() == 1 {
            let r = res.get(0).unwrap();
            let res = r.try_get::<f64, _>("num")
                .unwrap();
            let res = res.try_into()
                .expect("The convertion from signed number from database to unsigned local type failed.");
            return Ok(res);
        } else {
            return Err(Errors::ExpectOneResult { table_name: "elements_holding".to_string() });
        }
    }

    pub async fn change_element_holding(&self, handle: &ElementHandle, num: usize)
        -> Result<(), Errors> {
            let num: i64 = num.try_into()
                .expect("The convertion from local unsigned type to database's signed type failed.");
            let res = sqlx::query(
                "UPDATE elements_holding SET num=$1 WHERE name=$2"
            )
                .bind(num)
                .bind(handle.get_name())
                .execute(&self.database)
                .await?;
            if res.rows_affected() == 1 {
                return Ok(());
            } else {
                return Err(Errors::ExpectOneResult { table_name: "elements_holding".to_string() });
            }
        }

    pub async fn get_primary_elements(&self, ) -> Result<Vec<ElementHandle>, Errors> {
        let res = sqlx::query(
            "SELECT elements.name AS ename FROM elements LEFT JOIN recipes ON elements.name=recipes.name WHERE recipes.name IS NULL"
        )
            .fetch_all(&self.database)
            .await?;

        let mut v = vec![];
        for x in res.into_iter() {
            let a = x.try_get::<String, _>(0)?;
            v.push(ElementHandle::from(a));
        }

        Ok(v)
    }

    pub async fn is_primary_element(&self, handle: &ElementHandle) -> Result<bool, Errors> {
        let res = sqlx::query(
            "SELECT count(*) as num FROM recipes WHERE name=$1"
        )
            .bind(handle.get_name())
            .fetch_one(&self.database)
            .await?;
        let num = res.try_get::<i64, _>("num")
            .expect("Read count function's column `num` failed.");
        return Ok(num == 0);
    }

    pub async fn get_element_components(&self, handle: &ElementHandle)
        -> Result<(ElementHandle, ElementHandle), Errors> {
        //       let (component_a, component_b);
        let a: Vec<AnyRow> =
            sqlx::query("SELECT component_a,component_b FROM recipes WHERE name=?",)
            .bind(handle.get_name())
            .fetch_all(&self.database)
            .await?;

        if a.len() == 1 {
            let r = a.get(0).unwrap();
            let component_a: String = r
                .try_get("component_a")
                .unwrap();
            let component_b: String = r
                .try_get("component_b")
                .unwrap();
            return Ok((
                    ElementHandle::from(component_a),
                    ElementHandle::from(component_b)));
        } else if a.len() == 0 {
            return Err(
                Errors::FetchedZeroRow(handle.get_name())
            )
        } else {
            return Err(
                Errors::ExpectOneResult { table_name: "recipes".to_string() }
            )
        }
    }

    pub async fn get_what_component_can_build(&self, component: &ElementHandle)
        -> Result<Vec<ElementHandle>, Errors> {
        let mut res = Vec::new();
        let res1 = sqlx::query(
            "SELECT name FROM recipes WHERE component_a=$1"
        )
            .bind(component.get_name())
            .fetch_all(&self.database)
            .await?;

        res.extend(
            res1.iter().map(|a| a.try_get::<String, _>("name").unwrap())
        );

        let res1 = sqlx::query(
            "SELECT name FROM recipes WHERE component_b=$1"
        )
            .bind(component.get_name())
            .fetch_all(&self.database)
            .await?;

        res.extend(
            res1.iter().map(|a| a.try_get::<String, _>("name").unwrap())
        );
        Ok(res.iter().map(|a| ElementHandle::from(a.clone())).collect())
    }
}
