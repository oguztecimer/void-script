def soul():
    roam()
    
def roam():
    direction = 0
    pos = self.pos
    walk_distance = random(50,200)
    if (pos + walk_distance > 1000):
        direction = -1
    elif (pos < walk_distance):
        direction = 1
    else:
        if (random(0,2)) == 0:
            direction = 1
        else:
            direction = -1
    target_pos = direction * walk_distance + pos
    while (target_pos != self.pos):
        goto(target_pos)
    if (random(0,2)) == 0:
        wait_duration = random(20,200)
        for i in range(wait_duration):
            wait()

def goto(x):
    pos = self.pos
    if (pos == x): 
        return
    if (x < pos): 
        walk_left()
    else: 
        walk_right()