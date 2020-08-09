contract Contract {

	function or(bytes8 a, bytes8 b) public {
        bytes8 r = a | b;
	}

	function and(bytes8 a) public {
        bytes8 b = hex"41_42_43_44";
        bytes8 r = a & b;
	}

	function xor(bytes8 a) public {
        bytes8 b = "ABCD";
        bytes8 r = a ^ b;
	}
}
