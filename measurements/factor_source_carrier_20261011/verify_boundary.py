"""Verify the emitted factor pair against the exact source numeral."""

def verify_boundary(n, stdout):
    divisors = [int(line.split()[1]) for line in stdout.splitlines()
                if line.startswith('factor ')]
    quotients = [int(line.split()[1]) for line in stdout.splitlines()
                 if line.startswith('quotient ')]
    verified = (len(divisors) == len(quotients) == 1
                and 1 < divisors[0] < n
                and n % divisors[0] == 0
                and quotients[0] == n // divisors[0]
                and divisors[0] * quotients[0] == n)
    return divisors, quotients, verified
