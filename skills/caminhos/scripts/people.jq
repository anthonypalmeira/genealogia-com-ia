# The persons of a tree sweep, one per line, to look for targets by name, fact
# or place.
#
# Usage: jq -r -f scripts/people.jq <sweep file>
# Line: id, generation, every name the tree gives, then one field per fact:
# kind, value, date and place.

select(.pessoa)
| [.pessoa.id, .geracao, ([.pessoa.names[]?.nameForms[]?.fullText] | join(" / "))]
  + [.pessoa.facts[]?
     | [(.type | sub(".*/"; "")), .value, (.date | .original // .formal), .place.original]
     | map(select(. != null)) | join(" ")]
| @tsv
