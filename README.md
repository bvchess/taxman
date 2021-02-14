# taxman
A program to find optimal solutions to the taxman game.

![game 10 move 3](img/10.3.png)


Overview
---------------
Here are the rules of the game:
* Create a pot of numbers from 1 to N.
*	Make a move by choosing a number from the pot, removing it from the pot, and adding it to your score. The tax man
     then claims all other factors of your number that are in the pot and adds these factors to his score. You may not
     take a number unless the tax man will get to take at least one factor (you must always pay tax!)
* Continue taking numbers and paying tax until you cannot claim any more numbers.
* The game ends with the tax man taking all remaining unclaimed numbers. You win if the sum of the numbers you have
  claimed is larger than the sum of the numbers taken by the tax man.

With N = 5, a game could go like this:
* The player takes 5, and the tax man gets 1.
* The player takes 4, and the tax man gets 2.
* The player can’t take any more numbers, so the tax man gets everything left in the pot, which in this case is one
  number: 3.
The final scores are:  
  - Player: 5 + 4 = 9
  - Tax man: 1 + 2 + 3 = 6
    
The player wins by 3.

The game caught my attention as an interesting optimization problem. How large a game can a computer play and get the 
best possible score? There are N factorial potential sequences of moves, so the search space is large.  This project
can play an optimal game (achieve the best possible score) for values of N up to 683.  Past that I run out of patience
and/or CPU time.

Usage
---------------
`bin/taxman [options] <board size or range>`  

Where `board size or range` is an integer or range of integers.  Run the taxman command with no options in order to
learn more.

This command will show the optimal games from N=1 to
N=10:  
    `bin/taxman -s 1-10`  

This command will show debugging output as it searches for an optimal solution for N=450:  
    `bin/taxman -d 3 450`

Approach
---------------
In about 70% of cases an optimal solution for game N can be quickly derived from the optimal solution to game N-1.
For games where an optimal solution is not as easy to find, the program takes advantage of the fact that the maximum
feasible score for game N is no more than N larger than the optimal score for game N-1.  The program tries to find a
solution totaling the maximum feasible score and, if none can be found, targets a score one less than that.  It
proceeds downward until a solution can be found.

The program speeds up the search process for each potential solution total by decomposing the game into a series of simpler
games (where a number is allowed to be a factor or a composite but not both). Optimal solutions to these simpler
games can be found in polynomial time. Finding an optimal solution to the over-all game involves finding an optimal
set of "promotions" that elevate a number from being a factor in one game to being a move in the next game. Identifying
an optimal set of promotions takes an exponential amount of time in the worst case.

Implementation
---------------
The program includes pre-computed solutions to games from 1 to 701 so that it can make use of the solution to N-1
without having to compute it on every run.  These pre-computed solutions are available in JSON format
[here](src/main/resources/optimal.json).

I use the Java [BitSet](https://docs.oracle.com/en/java/javase/11/docs/api/java.base/java/util/BitSet.html) class for
fast operations on sets of small integers.  I use
Java [Streams](https://docs.oracle.com/en/java/javase/11/docs/api/java.base/java/util/stream/Stream.html)
and [ForkJoinPool](https://docs.oracle.com/en/java/javase/11/docs/api/java.base/java/util/concurrent/ForkJoinPool.html)
to take advantage of multi-core CPUs.

References and Links
---------------
- The On-Line Encyclopedia of Integer Sequences documents the Taxman Sequence (the optimal player score)
as [A019312](https://oeis.org/A019312).
- My son built an interactive version of Taxman for N = 16.  You can play it online here: <http://xvade.com/taxman>.
Other online implementations are <http://davidbau.com/archives/2008/12/07/taxman_game.html> and
<https://www.cryptool.org/en/cto/highlights/taxman>.
  

Acknowledgements
---------------
Thanks to EJ Technologies for the use of their excellent
[Java profiler](https://www.ej-technologies.com/products/jprofiler/overview.html).