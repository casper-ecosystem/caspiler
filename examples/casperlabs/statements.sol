contract Contract {

    function ifStm(bool a) public {
        if (a) {
            a = false;
        }
    }

    function ifElseStm(bool a) public {
        bool r;
        if (a) {
            r = false;
        } else {
            r = true;
        }
    }

    function ternary(bool a, bool b) public {
        uint256 r = a == b ? 1 : 2;
    }

    function whileFn(uint32 n) public {
        while (n >= 10) {
            n -= 9;
            while (n >= 1110) {
                n -= 92;
            }
            n -= 92111;
        }
        n = 40;
    }

    function forFn() public {
        uint n = 100000000;
        for (uint i = 0; i <= 10; i += 100) {
            n -= 92;
        }
        n += 1;
    }
}
