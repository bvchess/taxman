package org.taxman.h6.search;

import org.junit.jupiter.api.Test;
import org.taxman.h6.util.TxSet;

public class TaskDataTest {

    @Test
    void toBytesAndBack() {
        var td = new TaskData(10, TxSet.of(1, 2, 3), 25);
        var bytes = td.toByteArray();
        var td2 = TaskData.fromByteArray(bytes);
        assert td.equals(td2);
    }
}
