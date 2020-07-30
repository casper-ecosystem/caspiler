contract Contract {

	function not(bool a) public {
        bool r = !a;
	}

    function or(bool a, bool b) public {
        bool r = (a || b);
    }

    function and(bool a, bool b) public {
        bool r = (a && b);
    }

    function combined(bool a, bool b) public {
        bool r = !((true || b) && !b);
    }

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
}
