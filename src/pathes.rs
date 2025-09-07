use crate::recipes::ElementHandle;
use crate::dao::DAO;
use crate::math::{NumberMapToValue, Evaluable};
use crate::errors::*;

use std::cmp::Ordering;
use std::collections::{HashSet, HashMap};
use std::sync::{Arc, LazyLock};
use std::hash::Hash;

use snafu::prelude::*;
use ego_tree::Tree;

#[derive(Clone)]
pub struct Path {
    start: ElementHandle,
    end: ElementHandle,
    path: Vec<ElementHandle>,
    cached_weight: Option<f64>,
}

impl std::fmt::Debug for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}->", self.start.get_name())?;
        for x in &self.path {
            write!(f, "{}->", x.get_name())?;
        }
        write!(f, "{}", self.end.get_name())?;
        if let Some(weight) = self.cached_weight {
            write!(f, ": weight {}", weight)
        } else {
            write!(f, "")
        }

    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start
            && self.end == other.end
            && self.path == other.path
    }
}

impl Eq for Path {
}

impl Hash for Path {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.path.iter().for_each(|a| a.hash(state));
        self.end.hash(state);
    }
}


/// get the elements it can build and the components built it.
pub async fn get_relatives(dao: &DAO, ele: &ElementHandle) -> Result<HashSet<ElementHandle>> {
    use crate::dao::Errors;
    let mut relative_eles = HashSet::new();
    match dao.get_element_components(ele).await {
        Ok((component_a, component_b)) => {
            relative_eles.insert(component_a);
            relative_eles.insert(component_b);
        },
        Err(e) => {
            match e {
                Errors::FetchedZeroRow(_s) => {
                    // this situation means primary key
                    // do nothing
                }
                _ => {
                    return Err(
                        crate::errors::T4ACHError::Database {
                            err_loc: snafu::location!(),
                            backtrace: snafu::Backtrace::capture(),
                            source: e,
                        })
                }
            }
        }
    }
    let v = dao
        .get_what_component_can_build(ele)
        .await
        .context(DatabaseSnafu)?;

    relative_eles.extend(v);
    Ok(relative_eles)
}

pub async fn is_two_eles_connected(dao: &DAO, a: &ElementHandle, b: &ElementHandle)
    -> Result<bool> {
        let relative_eles = get_relatives(dao, a).await?;
        return Ok(relative_eles.contains(b));
}

impl Path {
    /// initialize a null pat
    pub fn new(start: ElementHandle, end: ElementHandle)
        -> Self {
            Self {
                start,
                end,
                path: Vec::new(),
                cached_weight: None,
            }
    }

    pub fn push(&mut self, ele: ElementHandle) {
        self.path.push(ele);
    }
    pub fn pop(&mut self, ) -> Option<ElementHandle> {
        self.path.pop()
    }
}

pub async fn is_path_viable(dao: &DAO, path: &Path) -> Result<bool> {
    return if path.path.is_empty() {
        is_two_eles_connected(dao, &path.start, &path.end).await
    } else {
        let a = {
            let mut v = vec![path.start.clone()];
            v.extend(path.path.clone());
            v.push(path.end.clone());
            v
        };
        for i in 0..a.len() - 1 {
            let x = a.get(i).unwrap();
            let y = a.get(i+1).unwrap();
            if !is_two_eles_connected(dao, x, y)
                .await? {
                    return Ok(false);
            }
        }
        Ok(true)
    }
}


pub async fn calc_path_steps_1(dao: Arc<DAO>, from: &ElementHandle, to: &ElementHandle)
    ->  Result<Vec<Path>> {
    let a_rel = get_relatives(dao.as_ref(), from).await?;
    let b_rel = get_relatives(dao.as_ref(), to).await?;
    let path_inners: Vec<ElementHandle> = a_rel.intersection(&b_rel)
        .cloned()
        .collect();

    let path = Path::new(from.clone(), to.clone());
    let mut ret = Vec::new();
    for path_inner in path_inners {
        let mut p = path.clone();
        p.push(path_inner);
        ret.push(p);
    }
    Ok(ret)
}

pub async fn calc_path_steps_2(dao: Arc<DAO>, from: &ElementHandle, to: &ElementHandle)
    -> Result<Vec<Path>> {
        let a_rel = get_relatives(dao.as_ref(), from).await?;
        let b_rel = get_relatives(dao.as_ref(), to).await?;

        let mut ret = Vec::new();

        for a in a_rel.iter() {
            for b in b_rel.iter() {
                if is_two_eles_connected(dao.as_ref(), a, b).await? {
                    let mut p = Path::new(from.clone(), to.clone());
                    p.push(a.clone());
                    p.push(b.clone());
                    ret.push(p);
                }
            }
        }

        Ok(ret)
}


static MAP_TO_VALUE: LazyLock<NumberMapToValue> = LazyLock::new(|| NumberMapToValue::default());
pub async fn calc_weight_single(dao: Arc<DAO>, ele: &ElementHandle) -> Result<f64> {
    let base_value = dao.get_element_base_value(ele).await.context(DatabaseSnafu)?;
    let element_holding = dao.get_element_num_holding(ele).await.context(DatabaseSnafu)?;
    let weight1 = MAP_TO_VALUE.eval(element_holding as f64).context(MathSnafu)?;
    let weight = weight1 / base_value;
    Ok(weight)
}

pub async fn crack_element_until_primary(dao: Arc<DAO>, ele: &ElementHandle) -> Result<HashMap<ElementHandle, usize>> {
    let tree = constructing_tree(dao, ele).await?;
    let mut ret = HashMap::new();
    tree.nodes().filter(|a| {
        !a.has_children() 
    })
    .for_each(|a| {
        if let Some(c) = ret.get_mut(a.value()) {
            *c += 1;
        } else {
            ret.insert(a.value().clone(), 1);
        }
    });

    Ok(ret)
}

async fn constructing_tree(dao: Arc<DAO>, ele: &ElementHandle) -> Result<Tree<ElementHandle>> {
    let mut tree = ego_tree::Tree::new(ele.clone());
    let pn = tree.root();
    use std::cell::RefCell;
    let level = RefCell::new(vec![pn.id()]);

    loop {
        let mut new_level = vec![];
        for nodeid in level.borrow().iter() {
            let mut pn = tree.get_mut(nodeid.clone()).unwrap();
            match dao.get_element_components(&pn.value()).await.context(DatabaseSnafu) {
                Ok((ca, cb)) => {
                    new_level.push(pn.append(ca).id());
                    new_level.push(pn.append(cb).id());
                },
                Err(e) => {
                    match e {
                        T4ACHError::Database { source, .. }
                        if matches!(source, crate::dao::Errors::FetchedZeroRow(..)) => {
                            // leaf node
                        },
                            _ => {
                                return Err(e);
                        }
                    }
                }
            }
        }
        if new_level.len() != 0 {
            level.swap(&RefCell::new(new_level));
        } else {
            break;
        }
    }
    Ok(tree)
}

/// An element's weight = map_to_value(element_holding) / base_value + (components' weight)
pub async fn calc_weight(dao: Arc<DAO>, ele: &ElementHandle) -> Result<f64> {
    let tree = constructing_tree(dao.clone(), ele).await?;

    let rate = 0.7f64;
    let mut weight = calc_weight_single(dao.clone(), tree.root().value()).await?;
    let mut sub_weight = 1f64;
    for x in tree.nodes() {
        if x != tree.root() {
            sub_weight += calc_weight_single(dao.clone(), x.value()).await?;
        }
    }
    weight = rate * weight + (1.0 - rate) * (1.0/sub_weight);
    Ok(weight)
}

pub async fn calc_weight_path(dao: Arc<DAO>, path: &Path) -> Result<f64> {
    let mut accumulated = 0f64;
    for x in &path.path {
        accumulated += calc_weight(dao.clone(), x).await?;
    }
    Ok(accumulated)
}

pub async fn calc_path_order_by_weight(dao: Arc<DAO>, from: &ElementHandle, to: &ElementHandle, steps_n: usize)
    -> Result<Vec<Path>> {
        let mut pathes = calc_path(dao.clone(), from, to, steps_n).await?;
        for path in &mut pathes {
            let weight = calc_weight_path(dao.clone(), path).await?;
            path.cached_weight = Some(weight);
        }
        pathes.sort_unstable_by(
            |a, b| {
                let av = a.cached_weight.unwrap();
                let bv = b.cached_weight.unwrap();
                // inverse less
                if av > bv {
                    Ordering::Less
                } else if av == bv {
                    Ordering::Equal
                } else {
                    Ordering::Greater
                }
            }
        );
        Ok(pathes)
}

pub async fn calc_path(dao: Arc<DAO>, from: &ElementHandle, to: &ElementHandle, steps_n: usize)
    -> Result<Vec<Path>> {
        if steps_n == 0 {
            if is_two_eles_connected(dao.as_ref(), from, to).await? {
                return Ok(vec![Path::new(from.clone(), to.clone())]);
            } else {
                return Ok(vec![]);
            }
        } else if steps_n == 1 {
            return calc_path_steps_1(dao.clone(), from, to).await;
        } else if steps_n == 2 {
            return calc_path_steps_2(dao.clone(), from, to).await;
        } else {
            let mut stack_f: Vec<Vec<ElementHandle>> = vec![vec![from.clone()]];
            let mut result_pathes = Vec::new();
            let end_relatives = get_relatives(dao.as_ref(), to).await?;

            'outer: loop {
                #[cfg(debug_assertions)]
                {
                    eprintln!("-- start");
                    for (i, x) in stack_f.iter().enumerate() {
                        eprintln!("--{i} - {x:?}");
                    }
                    eprintln!("-- end");
                }

                if let Some(last_v) = stack_f.last() {
                    // test if stepped on the last step.
                    if stack_f.len() - 1 != steps_n {
                        let p = last_v.last().unwrap();
                        let new_elements
                            = get_relatives(dao.as_ref(), p)
                            .await?
                            .iter()
                            .cloned()
                            .collect::<Vec<_>>();
                        // MARK push
                        stack_f.push(new_elements);
                    } else {
                        for x in last_v {
                            if end_relatives.contains(&x) {
                                let mut dest_path = Path::new(
                                    from.clone(),
                                    to.clone());


                                for x in 1..(stack_f.len() - 1) {
                                    let x = stack_f.get(x).unwrap();
                                    dest_path.push(x.last().unwrap().clone());
                                }
                                dest_path.push(x.clone());
                                result_pathes.push(dest_path);
                            }
                        }

                        stack_f.pop();
                        let stack_f_last_index = stack_f.len() - 1;
                        stack_f
                            .get_mut(stack_f_last_index)
                            .unwrap()
                            .pop();
                        if stack_f.last().unwrap().is_empty() {
                            stack_f.pop();

                            while let Some(v) = stack_f.last() {
                                if v.len() == 1 {
                                    stack_f.pop();
                                    if stack_f.is_empty() {
                                        break 'outer;
                                    }
                                    let stack_f_last_index = stack_f.len() - 1;
                                    stack_f
                                        .get_mut(stack_f_last_index)
                                        .unwrap()
                                        .pop();

                                    if stack_f.len() == 1 && stack_f.last().unwrap().len() == 0 {
                                        stack_f.pop();
                                    }
                                } else if v.len() == 0 {
                                    stack_f.pop();
                                } else {
                                    let stack_f_last_index = stack_f.len() - 1;
                                    stack_f
                                        .get_mut(stack_f_last_index)
                                        .unwrap()
                                        .pop();

                                    break;
                                }
                            }
                        }
                    }
                } else {
                    // stack_f is empty now.
                    break 'outer;
                }
            }
            return Ok(result_pathes);
        }
    }

#[cfg(test)]
mod tests {
    use crate::{dao::DAO, pathes::calc_path_order_by_weight, recipes::ElementHandle};

    use super::calc_path;

    use std::sync::{Arc, LazyLock};

    static INIT_SQLX_DRIVERS: LazyLock<()> = LazyLock::new(|| {
        sqlx::any::install_default_drivers();
    });

    #[tokio::test]
    async fn test_calc_path1() {
        let _ = &*INIT_SQLX_DRIVERS;

        let dao = Arc::new(DAO::new_str("sqlite://aspects.sqlite3").await);
        {
            let pathes = calc_path(dao.clone(), &ElementHandle::from("Aer"),
                &ElementHandle::from("Ignis"), 1).await.expect("1");
            // under 4.2.3.5 database
            assert_eq!(pathes.len(), 1usize);
            let p = pathes.get(0).unwrap();
            assert_eq!(p.path.get(0).unwrap().get_name(), "Lux")
        }
        {
            let pathes = calc_path(dao.clone(),
                &ElementHandle::from("Instrumentum"),
                &ElementHandle::from("Ignis"), 1).await.expect("1");
            // under 4.2.3.5 database
            assert_eq!(pathes.len(), 1usize);
            let p = pathes.get(0).unwrap();
            assert_eq!(p.path.get(0).unwrap().get_name(), "Telum")
        }
    }

    #[tokio::test]
    async fn test_calc_path2() {
        let _ = &*INIT_SQLX_DRIVERS;

        let dao = Arc::new(DAO::new_str("sqlite://aspects.sqlite3").await);
        {
            let pathes = calc_path(dao.clone(),
            &ElementHandle::from("Aer"),
            &ElementHandle::from("Ignis"),
            2).await.expect("1");
            assert_eq!(pathes.len(), 0);
        }
        {
            let pathes = calc_path(dao.clone(),
            &ElementHandle::from("Humanus"),
            &ElementHandle::from("Ignis"),
            2).await.expect("1");
            assert_eq!(format!("{pathes:?}"),
                "[Humanus->Instrumentum->Telum->Ignis]");
            // under 4.2.3.5 database
            /*
            assert_eq!(pathes.len(), 1usize);
            let p = pathes.get(0).unwrap();
            assert_eq!(p.path.get(0).unwrap().get_name(), "Lux")
            */
        }
        {
            let pathes = calc_path(dao.clone(),
            &ElementHandle::from("Machina"),
            &ElementHandle::from("Cognitio"),
            2).await.expect("1");
            assert_eq!(format!("{pathes:?}"), "[Machina->Instrumentum->Humanus->Cognitio]");
        }
        {
            use std::collections::HashSet;
            let pathes = calc_path(dao.clone(),
            &ElementHandle::from("Bestia"),
            &ElementHandle::from("Spiritus"),
            2).await.expect("1");
            let pathes_strs = pathes.iter()
                .map(|a| format!("{a:?}"))
                .collect::<HashSet<_>>();
            let right_strs =
                "Bestia->Humanus->Cognitio->Spiritus, Bestia->Victus->Mortuus->Spiritus, Bestia->Corpus->Mortuus->Spiritus"
    .split(", ")
    .map(|a| a.to_string())
    .collect::<HashSet<_>>();
            let res = &pathes_strs - &right_strs;
            assert!(res.is_empty(), "{pathes_strs:?}\n - \n{right_strs:?}\n = \n {res:?}");
        }
    }

    use super::is_path_viable;
    #[tokio::test]
    async fn test_calc_path3() {
        let _ = &*INIT_SQLX_DRIVERS;
        let dao = Arc::new(DAO::new_str("sqlite://aspects.sqlite3").await);
        {
            let pathes = calc_path(dao.clone(),
            &ElementHandle::from("Motus"),
            &ElementHandle::from("Mortuus"),
            3).await.expect("1");
            for x in &pathes {
                assert!(is_path_viable(dao.as_ref(), x).await.expect("bigger problem"), "{x:?} can't viable.");
            }
        }
        {
            let pathes = calc_path(dao.clone(),
            &ElementHandle::from("Perditio"),
            &ElementHandle::from("Motus"),
            3)
                .await.expect("1");
            // println!("finds {} ways: {pathes:?}", pathes.len(), );
            for x in &pathes {
                assert!(is_path_viable(dao.as_ref(), x).await.expect("bigger problem"), "{x:?} can't viable.");
            }
        }
    }

    #[tokio::test]
    async fn test_calc_path3_with_weight() {
        let _ = &*INIT_SQLX_DRIVERS;
        let dao = Arc::new(DAO::new_str("sqlite://aspects.sqlite3").await);
        {
            let pathes = calc_path_order_by_weight(dao.clone(),
            &ElementHandle::from("Motus"),
            &ElementHandle::from("Mortuus"),
            3).await.expect("1");
            println!("finds {} ways: {pathes:?}", pathes.len(), );
            for x in &pathes {
                assert!(is_path_viable(dao.as_ref(), x).await.expect("bigger problem"), "{x:?} can't viable.");
            }
        }
        {
            let pathes = calc_path_order_by_weight(dao.clone(),
            &ElementHandle::from("Perditio"),
            &ElementHandle::from("Motus"),
            3)
                .await.expect("1");
            println!("finds {} ways: {pathes:?}", pathes.len(), );
            for x in &pathes {
                assert!(is_path_viable(dao.as_ref(), x).await.expect("bigger problem"), "{x:?} can't viable.");
            }
        }
    }
}
