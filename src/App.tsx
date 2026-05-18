import "./App.css";
import ApiTester from "./features/apiTester/Dashboard";
import { ThemeProvider } from "./contexts/ThemeContext";

function App() {
  return (
    <ThemeProvider>
      <ApiTester />
    </ThemeProvider>
  );
}

export default App;
