from dataclasses import dataclass
from random import randint
from faker import Faker
fake = Faker()


def sign(num):
    return -1 if num < 0 else 1


def random_thruster():
    return Thruster(randint(-20, 20), randint(3, 20))


@dataclass
class Thruster:
    torque: float
    thrust: float
    throttle: float = 0.0

    def current_torque(self):
        return self.torque * self.throttle

    def current_thrust(self):
        return self.thrust * self.throttle

    def increase_throttle_to_counterbalance(self, torque):
        if self.throttle == 1.0 or sign(self.torque) == sign(torque):
            return torque

        req_increase_throttle = abs(torque) / abs(self.torque)
        old_torque = self.current_torque()
        self.throttle = self.throttle + req_increase_throttle
        new_torque = self.current_torque()
        return torque + (new_torque - old_torque)


THRUSTERS = {}

for i in range(randint(3, 6)):
    name = fake.name()
    thruster = random_thruster()
    THRUSTERS[name] = thruster


def current_torque():
    torque = 0
    for thruster in THRUSTERS.values():
        torque += thruster.current_torque()
    return torque


def current_thrust():
    thrust = 0
    for thruster in THRUSTERS.values():
        thrust += thruster.current_thrust()
    return thrust


def print_thrusters():
    for (name, thruster) in THRUSTERS.items():
        print(f"[{name}] {thruster}")
    print()
    print("Torque: ", current_torque())
    print("Thrust: ", current_thrust())
    print()


def unallocated_thrusters():
    return filter(lambda name: THRUSTERS[name].throttle < 1.0, THRUSTERS)


def find_smallest_mag_torque_thruster():
    unalloc = unallocated_thrusters()
    t = min(unalloc, key=lambda t: abs(THRUSTERS[t].torque))
    return (t, THRUSTERS[t])


def find_least_torque_thruster():
    unalloc = unallocated_thrusters()
    t = min(unalloc, key=lambda t: THRUSTERS[t].torque)
    return (t, THRUSTERS[t])


def find_most_torque_thruster():
    unalloc = unallocated_thrusters()
    t = max(unalloc, key=lambda t: THRUSTERS[t].torque)
    return (t, THRUSTERS[t])


def iterate_once():
    print("Begin iteration.")
    most_centered, thruster = find_smallest_mag_torque_thruster()

    print(f"Setting {most_centered} (torque {thruster.torque}) to 100%")
    THRUSTERS[most_centered].throttle = 1.0

    torque = current_torque()

    for name in unallocated_thrusters():
        if torque == 0.0:
            break
        t = THRUSTERS[name]
        if sign(t.torque) == sign(torque):
            continue
        torque = THRUSTERS[name].increase_throttle_to_counterbalance(torque)
        print(f"- After {name} (torque {THRUSTERS[name].torque}), {torque} torque remains")

    print_thrusters()


def current_max_throttle():
    name = max(THRUSTERS, key=lambda t: THRUSTERS[t].throttle)
    return THRUSTERS[name].throttle


print_thrusters()
print()
print()

for i in range(len(THRUSTERS) - 1):
    iterate_once()
    print()
    if current_torque() != 0.0:
        print("Stopping.\n")
        break

print("\nSolution:\n")

max_throttle = current_max_throttle()
for name in THRUSTERS:
    THRUSTERS[name].throttle /= max_throttle

print_thrusters()
