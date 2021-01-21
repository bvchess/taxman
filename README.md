# Taxman
Find optimal solutions to the taxman game

Overview
---------------
Here are the rules:
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

The game caught my attention as an interesting optimization problem. How large a game can a computer play? There are
N factorial potential sequences of moves, so the search space is large.  This project can play an optimal game (achieve the
best possible score) for values of N up to 683.  Past that I run out of patience and/or CPU time.

Usage
---------------
`bin/taxman [options] <board size or range>`  

Where `board size or range` is an integer or range of integers.  Run the taxman command with no options in order to learn more.

This command will show the optimal games from N=1 to
N=10:  
    `bin/taxman -s 1-10`  

This command will show debugging output as it looks for an optimal solution for N=450:  
    `bin/taxman -d 3 450`

Approach
---------------
In roughly 75% of cases, an optimal solution for game N can be quickly derived from the optimal solution to game N-1.
For games where an optimal solution is not as easy to find, the program takes advantage of the fact that the maximum
feasible score for game N is no more than N larger than the optimal score for game N-1.  The program tries to find a
solution totalling this maximum possible score and, if none can be found, looks for a score totalling N-2, and so on.

The program speeds up the search process for each potential solution by decomposing the game into a series of simpler
games (where a number is allowed to be a factor or a composite but not both) where an optimal solution can be found in
polynomial time and then finding an optimal set of "promotions" that elevate a number from being a factor in one
game to being a move in the next game.  Finding an optimal set of trades takes an exponential amount of time in the
worst case.

Implementation
---------------
I use the Java [BitSet](https://docs.oracle.com/en/java/javase/11/docs/api/java.base/java/util/BitSet.html) class for
fast operations on sets of small integers.  I use
Java [Streams](https://docs.oracle.com/en/java/javase/11/docs/api/java.base/java/util/stream/Stream.html)
and [ForkJoinPool](https://docs.oracle.com/en/java/javase/11/docs/api/java.base/java/util/concurrent/ForkJoinPool.html)
to take advantage of multi-core CPUs.

The program includes pre-computed solutions to games from 1 to 683 so that it can make use of the solution to N-1
without having to compute it on every run.  These pre-computed solutions are available in JSON format
[here](https://github.com/bvchess/taxman/blob/master/src/main/resources/optimal.json).

References & Links
---------------
- The On-Line Encyclopedia of Integer Sequences documents the Taxman Sequence (the optimal player score)
as [A019312](https://oeis.org/A019312).
- My son built an interactive version of Taxman for N = 16.  You can play it online here: <http://xvade.com/taxman>.
Other online implementations are <http://davidbau.com/archives/2008/12/07/taxman_game.html> and
<https://www.cryptool.org/en/cto/highlights/taxman>.