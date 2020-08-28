contract Contract {

    enum moons { Phobos, Deimos }

    enum planets { Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus, Neptune }

	function checkIfMercury(planets p) public {
        bool r = p == planets.Mercury;
	}

	function checkIfPhobos(moons m) public {
        bool r = m == moons.Phobos;
	}

	function checkIfPhobos2(moons m) public {
        bool r = m == moons.Phobos;
	}
}
