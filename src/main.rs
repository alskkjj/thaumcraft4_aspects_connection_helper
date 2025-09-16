mod recipes;
mod errors;
mod dao;
mod math;
mod pathes;

use std::sync::{Arc, LazyLock};

static INIT_SQLX_DRIVERS: LazyLock<()> = LazyLock::new(|| {
    sqlx::any::install_default_drivers();
});


use clap::{Parser, Subcommand};
use recipes::ElementHandle;

#[derive(Parser)]
#[command(about = "An aspects connector for Thaumcraft4", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Crack the aspects into its base aspects. Used to descript the base elements of a Node.
    /// the aspects array can be, for example a node with Sano*1, Aer*48, Ira*11 and Superbia*1,
    /// then it can be writen like this:
    ///  Sano Aer 48 Ira 11 Superbia
    Crack {
        #[arg(value_name="ASPECTS [QUANTITIES]")]
        aspects: Vec<String>,
    },
    /// Connect two elements with `steps_n` steps
    TryConnect {
        from: String,
        to: String,
        steps_n: usize,
    },
    /// List the elements in `Database`
    ListElements,
    /// List the recipes in `Database`
    ListRecipes,
    /// List the mods in `Database`
    ListMods,
    /// The `Aspects Connecting Algorithm` can calculate a `recommendation rate` by their
    /// quantities. This is the way let you input each one manually.
    ChangeElementHolding {
        element_name: String,
        change_to_num: usize,
    },
    /// List the elements currently holding.
    ListElementsHolding,
}

#[tokio::main]
async fn main() {
    let _ = &*INIT_SQLX_DRIVERS;
    let dao = Arc::new(dao::DAO::new_str("sqlite://aspects.sqlite3").await);
    let cli = Cli::parse();

    match &cli.command {
        Commands::ListElementsHolding => {
            let res = dao.list_elements_holding().await.expect("list_elements_holding failed.");
            res.iter()
                .for_each(|(e, f)| {
                    println!("Element: {} | Number: {:.0}", e.get_name(), f);
                })
        },
        Commands::ChangeElementHolding { element_name, change_to_num } => {
            let ele = ElementHandle::from(element_name.clone());
            dao.change_element_holding(&ele, *change_to_num).await
                .expect("Change Element Holding failed.");
        },
        Commands::ListMods => {
            let res = dao.list_mods().await.expect("list mods failed.");
            res.iter().for_each(|a| {
                println!("{}", a);
            })
        }
        Commands::ListRecipes => {
            let res
                = dao.list_recipes().await.expect("list recipes failed.");
            for (name, ca, cb) in res {
                println!("{} = {} + {}", name.get_name(), ca.get_name(), cb.get_name());
            }
            std::process::exit(0);
        },
        Commands::Crack { aspects } => {
            let insert_or_add =
                |mp: &mut HashMap<ElementHandle, usize>, eleh: ElementHandle, sz: usize| {
                    if let Some(ct) = mp.get_mut(&eleh) {
                        *ct += sz;
                    } else {
                        mp.insert(eleh, sz);
                    }
            };

            use std::collections::HashMap;
            let mut mp: HashMap<ElementHandle, usize> = HashMap::new();

            if aspects.len() == 0 {
                panic!("Must input at least one element.");
            }
            if aspects.get(0).unwrap().parse::<usize>().is_ok() {
                panic!("The first element in array must be an aspect.")
            }
            let mut idx = 0usize;
            while idx < aspects.len() {
                // idx is passed the break test
                let gt_str = aspects.get(idx).unwrap();
                let gt = ElementHandle::from(gt_str.clone());

                if idx + 1 < aspects.len() {
                    if dao.does_element_exists(&gt).await.expect("call does_element_exists failed") {
                        if let Ok(e) = aspects.get(idx+1).unwrap().parse::<usize>() {
                            insert_or_add(&mut mp, gt, e);
                            idx += 2;
                        } else {
                            insert_or_add(&mut mp, gt, 1usize);
                            idx += 1;
                        }
                    } else {
                        panic!("element {} doesn't exists.", gt_str);
                    }
                } else { // this is the last string.
                    if dao.does_element_exists(&gt).await.expect("call does_element_exists failed.") {
                        insert_or_add(&mut mp, gt, 1usize);
                        idx += 1;
                    } else {
                        panic!("element {} doesn't exists.", gt_str);
                    }
                }
            }
            let mut ret = HashMap::new();

            for aspect in &mp {
                for elee in
                    pathes::crack_element_until_primary(dao.clone(), aspect.0)
                        .await.expect("crack element until primary") {
                            insert_or_add(&mut ret, elee.0, elee.1 * aspect.1);
                        }
            }

            let mut vret = ret.iter().collect::<Vec<_>>();
            vret.sort_by(|a, b| {
                a.0.cmp(b.0)
            });
            for x in vret {
                println!("{}: {}", x.0.get_name(), x.1);
            }
        },
        Commands::TryConnect { from, to, steps_n } => {
            let from = recipes::ElementHandle::from(from.clone());
            let to = recipes::ElementHandle::from(to.clone());

            if !dao.does_element_exists(&from).await.expect("`does elements exists` failed") {
                eprintln!("The element {} doesn't exists", from.get_name());
                return;
            }
            if !dao.does_element_exists(&to).await.expect("`does elements exists` failed") {
                eprintln!("The element {} doesn't exists", to.get_name());
                return;
            }
            let pathes =
                pathes::calc_path_order_by_weight(dao.clone(), &from, &to, steps_n.clone()).await
                .expect("Calc pathes failed."); 

            if pathes.is_empty() {
                eprintln!("can't be connected");
            } else {
                for path in pathes {
                    println!("{:?}", path);
                }
            }
        }, 
        Commands::ListElements => {
            let v = dao.list_elements().await
                .expect("list elements error");
            for e in v {
                println!("{}", e.pretty_print());
            }
        }
    }
}
