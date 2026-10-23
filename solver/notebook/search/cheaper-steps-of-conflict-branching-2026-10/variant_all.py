import sys
p = sys.argv[1]
s = open(p).read()
old = """        let (above, here) = self.priced.split_at_mut(d);
        let parent = above.last().filter(|p| {
            carry
                && p.node == self.nodes[d - 1]
                && level.added.is_subset(&p.set)
                && p.set - level.added == last
                && level.live.is_subset(&p.live)
        });
        let mine = &mut here[0];
"""
new = """        let (above, here) = self.priced.split_at_mut(d);
        // Study (STEPCOST_CARRY_ALL=1): carry from the node's own last pricing, or else from the parent's, whatever
        // the sets, where the cycle holds; not exact.
        static ALL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        let all = carry && *ALL.get_or_init(|| std::env::var("STEPCOST_CARRY_ALL").is_ok_and(|v| v == "1"));
        let earlier = if all { Some(std::mem::take(&mut here[0])) } else { None };
        let same = earlier.as_ref().filter(|e| e.node == self.nodes[d]);
        let parent = above.last().filter(|p| {
            carry
                && p.node == self.nodes[d - 1]
                && (all
                    || (level.added.is_subset(&p.set)
                        && p.set - level.added == last
                        && level.live.is_subset(&p.live)))
        });
        let parent = same.or(parent);
        let mine = &mut here[0];
"""
assert s.count(old) == 1
s = s.replace(old, new)
open(p, 'w').write(s)
