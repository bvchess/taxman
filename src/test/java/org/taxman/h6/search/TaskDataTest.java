package org.taxman.h6.search;

import org.junit.jupiter.api.Test;
import org.taxman.h6.util.TxSet;

import java.io.IOException;

public class TaskDataTest {

    @Test
    void toBytesAndBack() throws IOException {
        var td = new TaskData(TxSet.of(1, 3), TxSet.of(1, 2, 3), 25);
        var bytes = td.toByteArray();
        var td2 = TaskData.fromByteArray(bytes);
        assert td.equals(td2);
    }
}
