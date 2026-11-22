# to control which compiler is used, set CXX= path-to-commpiler

CXXFLAGS= -O2 -Wall -std=c++11

.PHONY: target verify clean

EXE=				# on MS systems put .exe

CORE_OBJS= playtaxman.o heuristics.o printformats.o get_nvalues.o usage.o \
	math.o

GAMEBOARD_OBJS= common.o maxturn.o maxturnplus.o oddsevens.o onetax.o \
	greedyonetax.o 	maxpick.o

TAXMANGRAPH_OBJS= graphheuristics.o bornfree.o breakcycles.o findplayorder.o \
	maximum_weight_matching.o solvent.o

OBJS=	$(CORE_OBJS) $(GAMEBOARD_OBJS) $(TAXMANGRAPH_OBJS)

target: playtaxman$(EXE)

playtaxman$(EXE): $(OBJS)
	$(CXX) $(CXXFLAGS) $(LDFLAGS) $(OBJS) -o playtaxman$(EXE)

# verify correct output for N=21 for each heuristic
verify: target
	./runheuristics | diff - runheuristics.out


playtaxman.o: defs.h heuristics.h printformats.h gamedefs.h GameBoard.h

heuristics.o: heuristics.h

printformats.o: printformats.h

get_nvalues.o: defs.h

usage.o: defs.h heuristics.h printformats.h

math.o: defs.h

$(GAMEBOARD_OBJS): defs.h gamedefs.h heuristics.h GameBoard.h

$(TAXMANGRAPH_OBJS): defs.h heuristics.h gamedefs.h GameBoard.h graphdefs.h TaxmanGraph.h

clean:
	rm -f *.o

