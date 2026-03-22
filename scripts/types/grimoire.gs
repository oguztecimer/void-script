# Grimoire — runs every tick before entities
# No self, no position — use resource ops, queries, print
def soul():
    a = get_entities("summoner")
    set_var(a[0],"action_type",2)
