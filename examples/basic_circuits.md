=== self_hold ===
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/laddermd-cli convert tests/fixtures/self_hold.xml`
# Project: SelfHoldTest

## Program: Main

### Rung 1

LOGIC: Y001 = (X001 AND X002 OR Y001)

| Device | Type | LocalId |
|--------|------|---------|
| X001 | Contact(NO) | 2 |
| X002 | Contact(NO) | 3 |
| Y001 | Contact(NO) | 4 |
| Y001 | Coil | 5 |

```
|--[X001]--[X002]--+--(Y001)|
|--[Y001]--+        |
```

### Rung 2

LOGIC: Y001 (RESET) = X003

| Device | Type | LocalId |
|--------|------|---------|
| X003 | Contact(NO) | 7 |
| Y001 | Coil(R) | 8 |

```
|--[X003]--(R Y001)|
```


=== interlock ===
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running `target/debug/laddermd-cli convert tests/fixtures/interlock.xml`
# Project: InterlockTest

## Program: Main

### Rung 1

LOGIC: Y001 = X001 AND NOT X002

| Device | Type | LocalId |
|--------|------|---------|
| X001 | Contact(NO) | 2 |
| X002 | Contact(NC) | 3 |
| Y001 | Coil | 4 |

```
|--[X001]--[/X002]--(Y001)|
```

### Rung 2

LOGIC: Y002 = X002 AND NOT X001

| Device | Type | LocalId |
|--------|------|---------|
| X002 | Contact(NO) | 6 |
| X001 | Contact(NC) | 7 |
| Y002 | Coil | 8 |

```
|--[X002]--[/X001]--(Y002)|
```


=== timer ===
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/laddermd-cli convert tests/fixtures/timer.xml`
# Project: TimerTest

## Program: Main

### Rung 1

LOGIC: TON(T001_Instance) IN = X001

| Device | Type | LocalId |
|--------|------|---------|
| X001 | Contact(NO) | 2 |
| T001_Instance | Block(TON) | 3 |

```
|--[X001]--[TON T001_Instance]|
```

### Rung 2

LOGIC: Y001 = T001

| Device | Type | LocalId |
|--------|------|---------|
| T001 | Contact(NO) | 5 |
| Y001 | Coil | 6 |

```
|--[T001]--(Y001)|
```


=== emergency_stop ===
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running `target/debug/laddermd-cli convert tests/fixtures/emergency_stop.xml`
# Project: EmergencyStopTest

## Program: Main

### Rung 1

LOGIC: Y001 = (NOT X010 AND X001 OR NOT X010 AND Y001)

| Device | Type | LocalId |
|--------|------|---------|
| X010 | Contact(NC) | 2 |
| X001 | Contact(NO) | 3 |
| Y001 | Contact(NO) | 4 |
| Y001 | Coil | 5 |

```
|--[/X010]--[X001]--+--(Y001)|
|--[/X010]--[Y001]--+        |
```


