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
}
