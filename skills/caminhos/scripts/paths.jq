# Lists every path from the root of an ancestry sweep up to each target, one
# line per link, with the years the tree gives and a warning when the years of
# the child and of the parent cannot both be true. Years only: a hint about
# data that is itself a hypothesis.
#
# Usage: jq -rn -f scripts/paths.jq --arg alvos "ID,ID" <sweep file>
# A target that the sweep does not hold has no path and gives no line.
# Line: path number, target id, child, parent, warning (`-` when none).

# The year of the person's first fact of a kind (`Birth`, `Death`), or null.
def year($kind):
  [.facts[]? | select(.type | endswith("/" + $kind)) | .date
   | (.formal // .original // "") | scan("[0-9]{4}") | tonumber][0];

def summary:
  "\(.names[0].nameForms[0].fullText // "?") (\(.id)) \(year("Birth") // "?")–\(year("Death") // "?")";

# Why the years of a child and of a parent cannot both be true, or null.
def date_conflict($child; $parent):
  ($child | year("Birth")) as $born
  | ($parent | year("Birth")) as $parent_born
  | ($parent | year("Death")) as $parent_died
  # Who the parent is, the oldest age to have a child, and how many years
  # after the parent's death a child can still be born.
  | (if $parent.gender.type // "" | endswith("/Female") then ["a mãe", 55, 0] else ["o pai", 80, 1] end)
    as [$who, $oldest, $after_death]
  | if $born == null then null
    elif $parent_born != null and $born - $parent_born < 12 then
      "\($who) tinha menos de 12 anos"
    elif $parent_born != null and $born - $parent_born > $oldest then
      "\($who) tinha \($born - $parent_born) anos"
    elif $parent_died != null and $born - $parent_died > $after_death then
      "\($who) morreu \($born - $parent_died) anos antes"
    else null end;

[inputs] as $lines
| $lines[0].raiz as $root
| ($lines[1:] | INDEX(.pessoa.id)) as $tree
# The paths from a person down to the root. Only the children that the sweep
# holds are followed: the tree also gives the children outside the line. A
# person already on the path is skipped, so that a loop in the tree ends.
| def down($path):
    if $path[-1] == $root then $path
    else
      $tree[$path[-1]].filhos[]
      | select($tree[.] and (. as $id | $path | index($id) | not))
      | down($path + [.])
    end;
  [$alvos | scan("[^, ]+") | select($tree[.]) | down([.]) | reverse]
| to_entries[]
| .key as $n
| .value as $path
| range(($path | length) - 1)
| $tree[$path[.]].pessoa as $child
| $tree[$path[. + 1]].pessoa as $parent
| [$n + 1, $path[-1], ($child | summary), ($parent | summary), date_conflict($child; $parent) // "-"]
| @tsv
