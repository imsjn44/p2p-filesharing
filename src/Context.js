import React, { createContext, useState } from 'react';


export const MyContext = createContext();

export const Context = ({ children }) => {
  const [clickSearch, setClickSearch] = useState(false)
  const [transfers, setTransfer] = useState([]);
  const [searchResults, setSearchResults] = useState([]);
  const [downloadInfos, setDownloadInfos] = useState([]);
  
  const info = { 
    clickSearch,
    setClickSearch,
    transfers,
    setTransfer,
    setSearchResults,
    searchResults,
    downloadInfos,
    setDownloadInfos
   }
  return (
    <MyContext.Provider value={info}>
      {children}
    </MyContext.Provider>
  )
}

export default Context;