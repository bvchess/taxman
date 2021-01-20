package org.taxman.h6.frame;

import org.taxman.h6.game.Board;
import org.taxman.h6.bombus.Apiary;
import org.taxman.h6.bombus.Hive;
import org.taxman.h6.bombus.Namer;

import static org.taxman.h6.util.TxUnmodifiableSet.EmptySet;

import java.util.*;
import java.util.stream.Collectors;


public class FrameBuilder {
    final private Board board;
    final private Namer namer;

    private FrameBuilder(Board board) {
        this.board = board;
        this.namer = new Namer();
    }

    private Apiary makeApiary(Board b) {
        return new Apiary(b, EmptySet, namer);
    }

    private BaseFrame build() {
            Apiary apiary = makeApiary(board);

            Set<Hive> done = new HashSet<>();
            List<Hive> hives = apiary.hives().stream()
                    .filter(Hive::hasFreeFactors)
                    .collect(Collectors.toList());

            List<List<Hive>> levels = new ArrayList<>();
            List<Hive> level = hives.stream()
                    .filter(h -> h.downstream().stream().noneMatch(Hive::hasFreeFactors))  // start from sinks
                    .sorted(Comparator.comparingInt(h -> h.sources().min()))
                    .collect(Collectors.toList());

            while (!level.isEmpty()) {
                levels.add(level);
                done.addAll(level);
                level = level.stream()
                        .flatMap(h -> h.upstream().stream())
                        .filter(h -> !done.contains(h))
                        .filter(Hive::hasFreeFactors)
                        .distinct()
                        .sorted(Comparator.comparingInt(h -> h.sources().min()))
                        .collect(Collectors.toList());
            }

            BaseFrame result = new BaseFrame(levels.size() + 1);
            for(List<Hive> l: levels)
                result = result.addFrame(l);
            apiary.finishSetup();

            return result;
    }

    public static BaseFrame build(Board board) {
        return new FrameBuilder(board).build();
    }
}
