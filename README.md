# taxman
Find optimal solutions to the taxman game.

Here are the rules:
* Create a pot of numbers from 1 to N.
*	Make a move by choosing a number from the pot, removing it from the pot, and adding it to your score. The tax man then claims all other factors of your number that are in the pot and adds these factors to his score. You may not take a number unless the tax man will get to take at least one factor (you must always pay tax!)
* Continue taking numbers and paying tax until you cannot claim any more numbers.
* The game ends with the tax man taking all remaining unclaimed numbers. You win if the sum of the numbers you have claimed is larger than the sum of the numbers taken by the tax man.

With N = 5, a game could go like this:
* The player takes 5, and the tax man gets 1.
* The player takes 4, and the tax man gets 2.
* The player can’t take any more numbers, so the tax man gets everything left in the pot, which in this case is one number: 3.
The final scores are:
Player: 5 + 4 = 9
Tax man: 1 + 2 + 3 = 6
The player wins by 3.

My son built an interactive version of Taxman for N = 16.  You can play it online here: <http://xvade.com/taxman>

The game caught my attention as an interesting optimization problem. How large a game can a computer play? This project can play an optimal game (achieve the best possible score) for values of N up to 683.

Usage:  
`bin/taxman [options] <board size or range`  
Where `board size or range` is an integer or range of integers (examples: `1`, `20-25`).  Run the taxman command with no options in order to learn more.

Approach:  
In roughly 75% of cases, an optimal solution for game N can be quickly derived from the optimal solution to game N-1.  For games where an optimal solution is not as easy to find, the program takes advantage of the fact that the maximum feasible score for game N is no more than N larger than the optimal score for game N-1.  The program tries to find a solution totalling this maximum possible score and, if none can be found, looks for a score totalling N-2, and so on.

The program speeds up the search process for each potential solution by decomposing the game into a series of simpler games that can be played in polynomial time and then finding an optimal set of "trades" that move numbers between these games.
