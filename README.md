# thaumcraft4_aspects_connection_helper
A data-driven thaumcraft4 aspects connection calculator.

Pre-requests: rust development environment, sqlite3 command line tool

First of all, you must create the aspects's database, make sure you have the `sqlite3` command line client installed.

under the source root directory:
```console
  sqlite3 aspects.sqlite3
  # below is sqlite3 commands.
  .read sql/stage1.sql
  .read sql/aspects_4.2.3.5.sql
  ```

You can add the new aspects based on `aspects_4.2.3.5.sql`, remmber to add their `recipes` and `elements_holding`.

For details, see the `stage1.sql` for tables' defination, and `aspects_4.2.3.5.sql`'s comments for explanation.


For help: cargo run --release -- --help

For the help of a sub-command, for example, crack: cargo run --release -- crack --help

You can also copy the binary runnable file out from target directory, with database `aspects.sqlite3`.

# About the `Aspects Recommendation Algorithm`:

    It will calculate the recommendation rate for each path, the weight formulas:

    # `p` is a factor number to balance the node itself's weight and its sub_weight_sum.
    p = 0.7;
    function M: a function maps [0, infinity) to [0.0, 1.0), especially dense at domain [0, 1000). See `src/math.rs`
    weight_of_node_itself = M(h) / b;
    sub_weight_sum = sum_of_all_sub_components_weight;

    weight = p * weight_of_node_itself + (1-p) * (1.0 / sub_weight_sum)
    final_weight = The sum of the nodes' weight **between** the begin and the end

    The larger the `final_weight`, the higher the rank to be recommendated.

    If you are better on this mathematical problem, welcome to submit an issue.
