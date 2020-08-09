contract Contract {

	function add(uint256 a, uint256 b) public {
        uint256 r = a + b;
	}

    function sub(uint256 a, uint256 b) public {
        uint256 r = a - b;
	}

    function mul(uint256 a, uint256 b) public {
        uint256 r = a * b;
	}

    function div(uint256 a, uint256 b) public {
        uint256 r = a / b;
	}

    function powerOf(uint256 a, uint256 b) public {
        uint256 r = a ** b;
    }

    function modulo(uint256 a, uint256 b) public {
        uint256 r = a % b;
    }

    function addAssign(uint256 a, uint256 b) public {
        a += b;
    }

    function subAssign(uint256 a, uint256 b) public {
        a -= b;
    }

    function mulAssign(uint256 a, uint256 b) public {
        a *= b;
    }

    function divAssign(uint256 a, uint256 b) public {
        a /= b;
    }

    function modAssign(uint256 a, uint256 b) public {
        a %= b;
    }

    function unary(int32 a) public {
        int32 b = -a;
    }

    function preincrement(int32 a) public {
        int32 r = ++a;
    }

    function postincrement(int32 a) public {
        int32 r = a++;
    }

    function predecrement(int32 a) public {
        int32 r = --a;
    }

    function postdecrement(int32 a) public {
        int32 r = a--;
    }
}
