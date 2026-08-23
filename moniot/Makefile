# Build Robert Moniot's playtaxman.
#
# playtaxman.cpp is kept byte-for-byte as its author sent it, so the
# handful of headers it relies on transitively are forced in from here
# rather than edited into the source.  libc++ (macOS) pulls them in on
# its own; libstdc++ (most Linux) does not.
#
#   <algorithm>   sort, unique
#   <climits>     INT_MIN
#   <strings.h>   strcasecmp, rindex

CXX      ?= c++
CXXFLAGS ?= -std=c++17 -O2 -Wall
COMPAT    = -include algorithm -include climits -include strings.h

playtaxman: playtaxman.cpp
	$(CXX) $(CXXFLAGS) $(COMPAT) -o $@ $<

clean:
	rm -f playtaxman

.PHONY: clean
