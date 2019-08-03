# taxman
Find solutions to the taxman game.

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

The game caught my attention as an interesting optimization problem. How large a game can a computer play? What is the expected outcome? I wrote code that can play an optimal game (achieve the best possible score) for values of N up to 200. I coded a second approach I call G2 that can accommodate larger values of N but is not guaranteed to achieve the best possible score. On average G2 garners more than 99% of the optimal score and takes only a small fraction of the time required to play an optimal game.

Usage:  
`python taxman.py <board size or range> [solver name]`  
Where `board size or range` is an integer or range of integers (examples: `1`, `20-25`) and the optional solver name is one of `set`, `g2`, or `greedy`.
