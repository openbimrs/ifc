#!/bin/bash
cd /home/friedrich/occt-research/occt
EXT=( -name '*.cxx' -o -name '*.hxx' -o -name '*.h' -o -name '*.c' -o -name '*.lxx' -o -name '*.pxx' )

echo "== git HEAD =="
git log -1 --format='%H %ad'
echo
echo "== whole src: files / raw lines =="
find src -type f \( "${EXT[@]}" \) | wc -l
find src -type f \( "${EXT[@]}" \) -exec cat {} + | wc -l
echo
echo "== src excl Draw+Deprecated+GTests: files / raw lines =="
find src -type f \( "${EXT[@]}" \) -not -path 'src/Draw/*' -not -path 'src/Deprecated/*' -not -path '*/GTests/*' | wc -l
find src -type f \( "${EXT[@]}" \) -not -path 'src/Draw/*' -not -path 'src/Deprecated/*' -not -path '*/GTests/*' -exec cat {} + | wc -l
echo
echo "== GTests only: files / raw lines =="
find src -path '*/GTests/*' -type f \( "${EXT[@]}" \) | wc -l
find src -path '*/GTests/*' -type f \( "${EXT[@]}" \) -exec cat {} + | wc -l
echo
echo "== .hxx headers excl Draw/Deprecated/GTests (class proxy) =="
find src -name '*.hxx' -not -path 'src/Draw/*' -not -path 'src/Deprecated/*' -not -path '*/GTests/*' | wc -l
echo
echo "== leaf packages excl Draw/Deprecated/GTests =="
find src -mindepth 3 -maxdepth 3 -type d -not -path 'src/Draw/*' -not -name GTests | wc -l
echo
echo "== DEFINE_STANDARD_RTTIEXT occurrences (Transient-derived classes) =="
grep -rho 'DEFINE_STANDARD_RTTIEXT' --include='*.hxx' src | wc -l
echo
echo "== 'Handle(' textual usages across src =="
grep -rho 'Handle(' --include='*.cxx' --include='*.hxx' src | wc -l
echo
echo "== files mentioning Standard_Failure / throw =="
grep -rl 'Standard_Failure\|throw ' --include='*.cxx' --include='*.hxx' src | wc -l
echo
echo "== repo dir sizes =="
du -sh . src data tests dox
