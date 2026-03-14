# Miner Brain - Simple mining loop
# Finds the nearest asteroid, mines it, and deposits cargo

def main():
    while True:
        if cargo_full():
            # Return to mothership to deposit
            target = nearest(MOTHERSHIP)
            move(target)
            dock(target)
            deposit()
            undock()
        else:
            # Find and mine asteroids
            rock = nearest(ASTEROID)
            if rock is not None:
                move(rock)
                if can_mine():
                    mine()
                else:
                    # Asteroid depleted, find another
                    pass
            else:
                # No asteroids in range, expand search
                scan(50)
                wait()
