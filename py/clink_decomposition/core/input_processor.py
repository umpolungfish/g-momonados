"""
Input Processor: Calls CL8NK/CL9NK navigators and parses output
"""

import subprocess
import re
import os
from typing import Optional, List


# The 12 CLINK primitives
PRIMITIVE_SYMBOLS = ['⊢', '⊣', '≻', '≺', '⋈', '⊤', '∈', '∋', '⊙', '⊥', '⊞', '⊡']


class NavigatorOutput:
    """Output from CL8NK/CL9NK navigator"""
    def __init__(
        self,
        entry_name: str,
        navigator: str,
        clink_tuple: str = "",
        primitives: List[int] = None,
        primitive_symbols: List[str] = None,
        promoted_atoms: List[str] = None,
        tier: Optional[str] = None,
        distance: Optional[float] = None,
        full_output: str = "",
        reference_tuple: Optional[str] = None,
        match_count: Optional[int] = None,
        close_count: Optional[int] = None,
        distant_count: Optional[int] = None,
    ):
        self.entry_name = entry_name
        self.navigator = navigator
        self.clink_tuple = clink_tuple
        self.primitives = primitives or []
        self.primitive_symbols = primitive_symbols or []
        self.promoted_atoms = promoted_atoms or []
        self.tier = tier
        self.distance = distance
        self.full_output = full_output
        self.reference_tuple = reference_tuple
        self.match_count = match_count
        self.close_count = close_count
        self.distant_count = distant_count


class InputProcessor:
    """Processes catalog entries through CL8NK or CL9NK navigators"""
    
    def __init__(
        self, 
        cl8nk_path: Optional[str] = None,
        cl9nk_path: Optional[str] = None
    ):
        # Try to find the navigator scripts
        if cl8nk_path is None:
            cl8nk_path = self._find_navigator("cl8nk_navigator.py")
        if cl9nk_path is None:
            cl9nk_path = self._find_navigator("cl9nk_navigator.py")
        
        self.cl8nk_path = cl8nk_path or "cl8nk_navigator.py"
        self.cl9nk_path = cl9nk_path or "cl9nk_navigator.py"
    
    def _find_navigator(self, name: str) -> Optional[str]:
        """Find a navigator script in known locations"""
        # Check in imscribing_grammar/navigators
        base = "/home/mrnob0dy666/imsgct/imscribing_grammar/navigators"
        path = os.path.join(base, name)
        if os.path.isfile(path):
            return path
        
        # Check in current directory
        if os.path.isfile(name):
            return name
        
        # Check in PATH
        import shutil
        if shutil.which(name):
            return name
        
        return None
    
    def process_entry(self, entry_name: str, navigator: str = "cl8nk") -> NavigatorOutput:
        """
        Process a catalog entry through a navigator.
        
        Args:
            entry_name: Name of the catalog entry
            navigator: "cl8nk" or "cl9nk"
            
        Returns:
            NavigatorOutput with parsed results
        """
        if navigator not in ["cl8nk", "cl9nk"]:
            raise ValueError(f"Navigator must be 'cl8nk' or 'cl9nk', got '{navigator}'")
        
        # Execute the navigator command
        binary = self.cl8nk_path if navigator == "cl8nk" else self.cl9nk_path
        cmd = ["python3", binary, "entry", entry_name]
        
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=30,
                cwd=os.path.dirname(binary)
            )
            stdout = result.stdout
            stderr = result.stderr
            return_code = result.returncode
            
            if return_code != 0:
                # Try without python3
                cmd = [binary, "entry", entry_name]
                result = subprocess.run(
                    cmd,
                    capture_output=True,
                    text=True,
                    timeout=30,
                    cwd=os.path.dirname(binary)
                )
                stdout = result.stdout
                stderr = result.stderr
                return_code = result.returncode
                
                if return_code != 0:
                    raise RuntimeError(f"Navigator {navigator} failed with code {return_code}: {stderr}")
            
            # Parse the output
            return self._parse_navigator_output(entry_name, navigator, stdout, stderr)
            
        except subprocess.TimeoutExpired:
            raise RuntimeError(f"Navigator {navigator} timed out for entry '{entry_name}'")
        except FileNotFoundError as e:
            raise RuntimeError(f"Navigator {navigator} not found. Tried: {binary}. Error: {e}")
    
    def _parse_navigator_output(
        self, 
        entry_name: str, 
        navigator: str, 
        stdout: str, 
        stderr: str
    ) -> NavigatorOutput:
        """Parse CL8NK/CL9NK output into structured data"""
        
        # Try to extract the CLINK tuple
        tuple_match = re.search(r'⟨([^⟩]*)⟩', stdout)
        clink_tuple = tuple_match.group(0) if tuple_match else ""
        
        # Extract primitives from the tuple
        primitives = []
        primitive_symbols = []
        if clink_tuple:
            # Remove the angle brackets
            tuple_content = clink_tuple[1:-1]
            # Split by the Unicode primitives - they're separated by dots in the tuple
            parts = tuple_content.split('⋅')
            for part in parts:
                part = part.strip()
                if part in PRIMITIVE_SYMBOLS:
                    primitives.append(PRIMITIVE_SYMBOLS.index(part))
                    primitive_symbols.append(part)
        
        # If we didn't get 12 primitives, try to find them in the Prim Value table
        if len(primitives) != 12:
            primitives, primitive_symbols = self._extract_primitives_from_table(stdout)
        
        # If still not 12, try alternative parsing
        if len(primitives) != 12:
            primitives, primitive_symbols = self._extract_primitives_alternative(stdout)
        
        # Extract promoted atoms
        promoted_atoms = self._extract_promoted_atoms(stdout)
        
        # Extract tier
        tier = self._extract_tier(stdout)
        
        # Extract distance
        distance = self._extract_distance(stdout)
        
        # Extract reference tuple
        reference_tuple = self._extract_reference_tuple(stdout)
        
        # Extract match/close/distant counts
        match_count = self._extract_count(stdout, r'match:(\d+)')
        close_count = self._extract_count(stdout, r'close:(\d+)')
        distant_count = self._extract_count(stdout, r'distant:(\d+)')
        
        return NavigatorOutput(
            entry_name=entry_name,
            navigator=navigator,
            clink_tuple=clink_tuple,
            primitives=primitives,
            primitive_symbols=primitive_symbols,
            promoted_atoms=promoted_atoms,
            tier=tier,
            distance=distance,
            full_output=stdout,
            reference_tuple=reference_tuple,
            match_count=match_count,
            close_count=close_count,
            distant_count=distant_count
        )
    
    def _extract_primitives_from_table(self, stdout: str):
        """Extract primitives from the Prim Value table"""
        primitives = []
        primitive_symbols = []
        
        # Look for lines with primitive symbols
        pattern = r'(\⊢|\⊣|\≻|\≺|\⋈|\⊤|\∈|\∋|\⊙|\⊥|\⊞|\⊡)'
        matches = re.findall(pattern, stdout)
        
        for symbol in matches:
            if symbol in PRIMITIVE_SYMBOLS:
                idx = PRIMITIVE_SYMBOLS.index(symbol)
                if idx not in primitives:
                    primitives.append(idx)
                    primitive_symbols.append(symbol)
        
        return primitives, primitive_symbols
    
    def _extract_primitives_alternative(self, stdout: str):
        """Alternative method to extract primitives from output"""
        primitives = []
        primitive_symbols = []
        
        # Look for the reference tuple line
        match = re.search(r'Reference[^⟨]*⟨([^⟩]*)⟩', stdout)
        if match:
            tuple_content = match.group(1)
            parts = tuple_content.split('⋅')
            for part in parts:
                part = part.strip()
                if part in PRIMITIVE_SYMBOLS:
                    idx = PRIMITIVE_SYMBOLS.index(part)
                    if idx not in primitives:
                        primitives.append(idx)
                        primitive_symbols.append(part)
        
        return primitives, primitive_symbols
    
    def _extract_promoted_atoms(self, stdout: str) -> List[str]:
        """Extract promoted atoms from output"""
        atoms = []
        # Look for lines like "promoted atoms: PRIME_POINT, BROADCAST_TRANSCENDENCE, PHI_C"
        match = re.search(r'promoted atoms:\s*(.+)', stdout, re.IGNORECASE)
        if match:
            atoms_str = match.group(1).strip()
            atoms = [a.strip() for a in atoms_str.split(',')]
        return atoms
    
    def _extract_tier(self, stdout: str) -> Optional[str]:
        """Extract tier from output"""
        match = re.search(r'tier:\s*(\S+)', stdout, re.IGNORECASE)
        return match.group(1) if match else None
    
    def _extract_distance(self, stdout: str) -> Optional[float]:
        """Extract distance from output"""
        match = re.search(r'd\([^)]+\):\s*([\d.]+)', stdout)
        if match:
            return float(match.group(1))
        return None
    
    def _extract_reference_tuple(self, stdout: str) -> Optional[str]:
        """Extract reference tuple from output"""
        match = re.search(r'Reference[^⟨]*⟨([^⟩]*)⟩', stdout)
        if match:
            return f"⟨{match.group(1)}⟩"
        return None
    
    def _extract_count(self, stdout: str, pattern: str) -> Optional[int]:
        """Extract a count from output"""
        match = re.search(pattern, stdout, re.IGNORECASE)
        return int(match.group(1)) if match else None
    
    def test_navigator(self, navigator: str = "cl8nk") -> bool:
        """Test if a navigator is available and working"""
        try:
            binary = self.cl8nk_path if navigator == "cl8nk" else self.cl9nk_path
            cmd = ["python3", binary, "--help"]
            result = subprocess.run(cmd, capture_output=True, timeout=10, cwd=os.path.dirname(binary))
            return result.returncode == 0
        except:
            return False
