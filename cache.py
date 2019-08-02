import collections


class Cache:
    """
    An LRU cache for memoizing solver results based on collections.OrderedDict.
    Set max size in constructor or 0 for unlimited size.
    """
    def __init__(self, max_size=0):
        self._max_size = max_size
        self._entries = collections.OrderedDict()
        self._lookups = 0
        self._hits = 0
        self._tripmeter_lookups = 0
        self._tripmeter_hits = 0
        self._tripmeter_starting_size = 0

        # If true, we'll keep the actual key around for later evaluation, if false, we only hash.  Keeping keys
        # around can take a lot of memory.
        self._keep_key = False

    def reset_tripmeter(self):
        self._tripmeter_hits = 0
        self._tripmeter_lookups = 0
        self._tripmeter_starting_size = len(self._entries)

    def _make_key(self, key):
        if self._keep_key:
            return key
        else:
            return hash(key)

    def _make_value(self, value):
        if self._keep_key:
            return 0, value
        else:
            return value

    def update_value(self, value):
        if self._keep_key:
            return value[0]+1, value[1]
        else:
            return value

    def get(self, key, otherwise=None):
        self._lookups += 1
        self._tripmeter_lookups += 1
        real_key = self._make_key(key)
        if real_key in self._entries:
            self._hits += 1
            self._tripmeter_hits += 1
            real_value = self.update_value(self._entries.pop(real_key))
            self._entries[real_key] = real_value
            if self._keep_key:
                return real_value[1]
            else:
                return real_value
        else:
            return otherwise

    def set(self, key, value):
        real_key = self._make_key(key)
        real_value = self._make_value(value)
        if real_key in self._entries:
            self._entries.pop(real_key)
        while self._max_size and len(self._entries) >= self._max_size:
            self._entries.popitem(last=False)
        self._entries[real_key] = real_value

    def stats(self):
        hit_rate = (float(self._tripmeter_hits * 100) / self._tripmeter_lookups) if self._tripmeter_lookups else 0
        growth = len(self._entries) - self._tripmeter_starting_size
        fmt = "{:,} cache hits out of {:,} lookups ({:.2f}% hit rate) with {:,} entry cache growth"
        return fmt.format(self._tripmeter_hits, self._tripmeter_lookups, hit_rate, growth)

    def extended_stats(self):
        top = ""
        if self._keep_key:
            report_size = 10
            top = "\nmost used cache entries:\n"
            for key in sorted(self._entries.keys(), reverse=True, key=lambda x: self._entries[x][0])[0:report_size]:
                top += "\t{:6,} uses of key {}\n".format(self._entries[key][0], key)

        hit_rate = (float(self._hits * 100) / self._lookups) if self._lookups else 0
        size = len(self._entries)
        fmt = "{:,} cache hits out of {:,} lookups ({:.2f}% hit rate) with {:,} entries"
        stats = fmt.format(self._hits, self._lookups, hit_rate, size)
        return "{}{}".format(stats, top)

    def size(self):
        return len(self._entries)

    def clear(self):
        self._entries.clear()
        # should I also clear hits and lookups?  Not sure what the caller's expectation might be
